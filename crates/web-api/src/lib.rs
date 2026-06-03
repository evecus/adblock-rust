//! Minimal HTTP/JSON API server using tiny_http.
//! Runs in a dedicated thread pool (not tokio), communicates with the
//! async DNS engine via Arc<AppStateApi>.

use std::sync::Arc;
use std::thread;

use anyhow::Result;
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};
use tracing::{info, warn};

pub trait AppStateApi: Send + Sync + 'static {
    fn query_recent(&self, n: usize, filter: Option<&str>) -> Vec<Value>;
    fn query_stats(&self) -> Value;
    fn engine_metadata(&self) -> Option<Value>;
    fn cache_len(&self) -> usize;
    fn test_domain(&self, domain: &str) -> String;
    fn reload_rules(&self) -> Result<()>;
    fn get_config(&self) -> Value;
    fn update_config(&self, v: Value) -> Result<()>;
    fn get_rulesets(&self) -> Vec<Value>;
    fn toggle_ruleset(&self, name: &str, enabled: bool) -> Result<()>;
    fn add_custom_rule(&self, rule: &str) -> Result<()>;
    fn remove_custom_rule(&self, rule: &str) -> Result<()>;
    fn get_custom_rules(&self) -> Vec<String>;
}

pub fn run_web_server<S: AppStateApi>(state: Arc<S>, bind: &str) -> Result<()> {
    let server = Server::http(bind)
        .map_err(|e| anyhow::anyhow!("HTTP bind failed: {}", e))?;
    info!(bind = %bind, "Web API listening");

    // Use 4 threads for HTTP handling
    let server = Arc::new(server);
    let mut handles = vec![];
    for _ in 0..4 {
        let server = server.clone();
        let state = state.clone();
        let h = thread::spawn(move || {
            for req in server.incoming_requests() {
                handle_request(req, &state);
            }
        });
        handles.push(h);
    }
    for h in handles {
        let _ = h.join();
    }
    Ok(())
}

fn handle_request<S: AppStateApi>(mut req: Request, state: &Arc<S>) {
    let method = req.method().clone();
    let url = req.url().to_string();

    // Parse path and query string
    let (path, query) = match url.find('?') {
        Some(i) => (&url[..i], &url[i + 1..]),
        None => (url.as_str(), ""),
    };

    let params = parse_qs(query);

    let result = match (method, path) {
        // ── Stats ────────────────────────────────────────────────────────────
        (Method::Get, "/api/stats") => {
            let stats = state.query_stats();
            let meta = state.engine_metadata();
            Ok(json!({
                "queries": stats,
                "engine": meta,
                "cache": { "size": state.cache_len() }
            }))
        }

        // ── Query log ────────────────────────────────────────────────────────
        (Method::Get, "/api/queries") => {
            let limit = params.get("limit")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(100)
                .min(1000);
            let filter = params.get("domain").map(|s| s.as_str());
            Ok(json!(state.query_recent(limit, filter)))
        }

        // ── Test domain ──────────────────────────────────────────────────────
        (Method::Get, "/api/test") => {
            match params.get("domain") {
                Some(d) => Ok(json!({ "domain": d, "result": state.test_domain(d) })),
                None => Err("missing ?domain=".to_string()),
            }
        }

        // ── Reload rules ─────────────────────────────────────────────────────
        (Method::Post, "/api/rules/reload") => {
            state.reload_rules()
                .map(|_| json!({ "ok": true }))
                .map_err(|e| e.to_string())
        }

        // ── Rulesets list ────────────────────────────────────────────────────
        (Method::Get, "/api/rules/rulesets") => {
            Ok(json!(state.get_rulesets()))
        }

        // ── Toggle ruleset: PUT /api/rules/rulesets/<name>/toggle ────────────
        (Method::Put, p) if p.starts_with("/api/rules/rulesets/") && p.ends_with("/toggle") => {
            let name = p
                .trim_start_matches("/api/rules/rulesets/")
                .trim_end_matches("/toggle");
            let body = read_body_json(&mut req);
            let enabled = body
                .and_then(|v| v.get("enabled").and_then(|e| e.as_bool()))
                .unwrap_or(true);
            state.toggle_ruleset(name, enabled)
                .map(|_| json!({ "ok": true }))
                .map_err(|e| e.to_string())
        }

        // ── Custom rules ─────────────────────────────────────────────────────
        (Method::Get, "/api/rules/custom") => {
            Ok(json!(state.get_custom_rules()))
        }
        (Method::Post, "/api/rules/custom") => {
            let body = read_body_json(&mut req);
            match body.and_then(|v| v.get("rule").and_then(|r| r.as_str()).map(|s| s.to_string())) {
                Some(rule) => state.add_custom_rule(&rule)
                    .map(|_| json!({ "ok": true }))
                    .map_err(|e| e.to_string()),
                None => Err("missing field: rule".into()),
            }
        }
        (Method::Delete, "/api/rules/custom") => {
            let body = read_body_json(&mut req);
            match body.and_then(|v| v.get("rule").and_then(|r| r.as_str()).map(|s| s.to_string())) {
                Some(rule) => state.remove_custom_rule(&rule)
                    .map(|_| json!({ "ok": true }))
                    .map_err(|e| e.to_string()),
                None => Err("missing field: rule".into()),
            }
        }

        // ── Config ───────────────────────────────────────────────────────────
        (Method::Get, "/api/config") => Ok(state.get_config()),
        (Method::Put, "/api/config") => {
            let body = read_body_json(&mut req);
            match body {
                Some(v) => state.update_config(v)
                    .map(|_| json!({ "ok": true }))
                    .map_err(|e| e.to_string()),
                None => Err("invalid JSON body".into()),
            }
        }

        // ── Catch-all / frontend ─────────────────────────────────────────────
        (Method::Get, "/") | (Method::Get, "/index.html") => {
            send_html(req, FRONTEND_HTML);
            return;
        }
        (Method::Get, p) if p.starts_with("/assets/") => {
            send_404(req);
            return;
        }

        // ── CORS preflight ───────────────────────────────────────────────────
        (Method::Options, _) => {
            let _ = req.respond(
                Response::empty(204)
                    .with_header(cors_header())
                    .with_header(cors_allow_methods())
            );
            return;
        }

        _ => Err(format!("Not found: {path}")),
    };

    match result {
        Ok(v) => send_json(req, 200, &v),
        Err(e) => send_json(req, 400, &json!({ "error": e })),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn send_json(req: Request, status: u16, v: &Value) {
    let body = v.to_string();
    let resp = Response::from_string(body)
        .with_status_code(status)
        .with_header(json_header())
        .with_header(cors_header());
    if let Err(e) = req.respond(resp) {
        warn!("HTTP response error: {}", e);
    }
}

fn send_html(req: Request, html: &'static str) {
    let resp = Response::from_string(html)
        .with_status_code(200)
        .with_header(html_header())
        .with_header(cors_header());
    let _ = req.respond(resp);
}

fn send_404(req: Request) {
    let _ = req.respond(Response::empty(404).with_header(cors_header()));
}

fn read_body_json(req: &mut Request) -> Option<Value> {
    let mut body = String::new();
    req.as_reader().read_to_string(&mut body).ok()?;
    serde_json::from_str(&body).ok()
}

fn parse_qs(qs: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(
                url_decode(k),
                url_decode(v),
            );
        }
    }
    map
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i+1]), hex_val(bytes[i+2])) {
                out.push(char::from(h << 4 | l));
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
            continue;
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn json_header() -> Header {
    Header::from_bytes("Content-Type", "application/json").unwrap()
}
fn html_header() -> Header {
    Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap()
}
fn cors_header() -> Header {
    Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap()
}
fn cors_allow_methods() -> Header {
    Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS").unwrap()
}

// ── Embedded fallback HTML (replaced by build script with real frontend) ─────
static FRONTEND_HTML: &str = include_str!("frontend.html");
