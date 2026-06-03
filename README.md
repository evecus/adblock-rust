# dns-filter

一个用 Rust 编写的轻量级 DNS 过滤工具，使用高效的二进制规则集 `.ars` 格式。

## 功能特性

- **高性能匹配**：FST（有限状态转换器）精确/后缀匹配 + Aho-Corasick 关键词 + RegexSet
- **二进制规则集**：`.ars` 格式，zstd 压缩，毫秒级加载百万条规则
- **规则转换**：支持从 AdGuardHome TXT 规则、Mihomo YAML 转换
- **热重载**：API 触发，无需重启服务
- **Web 管理界面**：查询日志、规则管理、域名测试、实时统计
- **多架构**：Linux amd64 / arm64 静态二进制
- **DNS 缓存**：内置 LRU 缓存，TTL 自动失效

## 快速开始

### 下载预编译二进制

从 [Releases](../../releases) 页面下载对应平台的压缩包：

```bash
# Linux x86_64
wget https://github.com/you/dns-filter/releases/latest/download/dns-filter-v1.0.0-linux-amd64.tar.gz
tar -xzf dns-filter-*.tar.gz
```

### 转换规则文件

```bash
# 从 AdGuardHome 规则列表转换
./ars-convert convert easylist.txt -o rules.ars

# 从 Mihomo YAML 转换
./ars-convert convert reject.yaml -o rules.ars

# 合并多个规则文件
./ars-convert convert list1.txt list2.yaml -o combined.ars

# 查看规则集信息
./ars-convert info rules.ars
```

### 运行

```bash
# 编辑配置文件
cp config.example.json config.json
# 修改 rulesets[0].path 指向你的 rules.ars 文件

# 运行（绑定 53 端口需要 root 或 CAP_NET_BIND_SERVICE）
sudo ./dns-filter --config config.json

# 或使用更高端口测试（无需 root）
# 先修改 config.json: "bind": "127.0.0.1:5353"
./dns-filter --config config.json

# 测试
dig @127.0.0.1 -p 5353 ads.example.com
```

Web 管理界面默认在 `http://localhost:3000`

## 配置说明

```json
{
  "dns": {
    "bind": "0.0.0.0:53",
    "block_mode": "nxdomain",   // nxdomain | zeroip | refused
    "query_log_size": 10000,    // 保留最近查询条数
    "block_ttl": 3600,          // 拦截响应 TTL（秒）
    "cache_size": 50000         // DNS 缓存条目数
  },
  "upstream": {
    "servers": ["8.8.8.8:53", "8.8.4.4:53"],
    "timeout_ms": 3000,
    "failover": true            // 上游失败时切换到下一个
  },
  "rulesets": [
    {
      "name": "Default Blocklist",
      "path": "/etc/dns-filter/rules/default.ars",
      "enabled": true
    }
  ],
  "web": {
    "bind": "0.0.0.0:3000"
  },
  "log": { "level": "info" }
}
```

## .ars 规则集格式

### 文件结构

```
[Header 32 bytes]  Magic + Version + RuleCount + SectionCount + CRC32
[Section*]         每个 Section：ID(1) + Codec(1) + CompLen(4) + RawLen(4) + Data
[Trailer 4 bytes]  整个文件的 CRC32
```

### Section 类型

| ID   | 内容 | 存储结构 |
|------|------|---------|
| 0x01 | 精确拦截域名 | FST Set |
| 0x02 | 后缀拦截域名 | FST Set（标签倒序） |
| 0x03 | 关键词拦截   | Aho-Corasick 模式串 |
| 0x04 | 正则拦截     | 换行分隔的正则字符串 |
| 0x11 | 精确白名单   | FST Set |
| 0x12 | 后缀白名单   | FST Set（标签倒序） |
| 0x20 | 重写规则     | JSON 数组 |
| 0x40 | 元数据       | JSON |

### 匹配优先级

```
1. 自定义规则（API 动态添加）
2. 白名单精确 → 白名单后缀 → 白名单关键词 → 白名单正则
3. 重写规则
4. 拦截精确 → 拦截后缀 → 拦截关键词 → 拦截正则
5. NoMatch → 转发上游
```

### 支持的规则语法

**AdGuardHome TXT**:
```
||example.com^          # 拦截 example.com 及所有子域名
@@||safe.com^           # 白名单
0.0.0.0 tracker.com    # hosts 格式
/^ads\./               # 正则
```

**Mihomo YAML**:
```yaml
payload:
  - +.example.com          # 后缀拦截（同 DOMAIN-SUFFIX）
  - DOMAIN,exact.com       # 精确拦截
  - DOMAIN-SUFFIX,ads.com  # 后缀拦截
  - DOMAIN-KEYWORD,tracker # 关键词拦截
  - DOMAIN-REGEX,^ads\.    # 正则拦截
```

## Web API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET  | `/api/stats` | 统计信息（查询数、拦截数、缓存大小） |
| GET  | `/api/queries?limit=100&domain=filter` | 查询日志 |
| GET  | `/api/test?domain=example.com` | 测试域名匹配结果 |
| POST | `/api/rules/reload` | 热重载规则集 |
| GET  | `/api/rules/rulesets` | 规则集列表 |
| PUT  | `/api/rules/rulesets/:name/toggle` | 启用/禁用规则集 |
| GET  | `/api/rules/custom` | 自定义规则列表 |
| POST | `/api/rules/custom` | 添加自定义规则 `{"rule": "||ads.com^"}` |
| DELETE | `/api/rules/custom` | 删除自定义规则 |
| GET  | `/api/config` | 当前配置 |
| PUT  | `/api/config` | 更新配置 |

## Docker

```bash
# 使用 Docker Compose
docker run -d \
  --name dns-filter \
  -p 53:53/udp \
  -p 3000:3000 \
  -v /etc/dns-filter:/etc/dns-filter \
  ghcr.io/you/dns-filter:latest
```

### docker-compose.yml

```yaml
version: '3.8'
services:
  dns-filter:
    image: ghcr.io/you/dns-filter:latest
    container_name: dns-filter
    restart: unless-stopped
    ports:
      - "53:53/udp"
      - "53:53/tcp"
      - "3000:3000"
    volumes:
      - ./config.json:/etc/dns-filter/config.json:ro
      - ./rules:/etc/dns-filter/rules:ro
    cap_add:
      - NET_BIND_SERVICE
```

## 从源码构建

```bash
# 需要 Rust >= 1.85
git clone https://github.com/you/dns-filter
cd dns-filter

# 构建前端
cd frontend && npm ci && npm run build && cd ..

# 本地 release 构建
cargo build --release

# 交叉编译（需要 cross）
cargo install cross
make build-amd64
make build-arm64
```

## 性能参考

| 操作 | 规模 | 耗时 |
|------|------|------|
| 加载 100 万条规则 | .ars 文件 ~8MB | < 200ms |
| 精确/后缀匹配     | FST 查找       | O(域名长度) |
| 关键词匹配        | Aho-Corasick   | O(域名长度) |
| DNS 查询处理      | 命中缓存        | < 0.1ms |
| DNS 查询处理      | 命中规则拦截    | < 0.5ms |
| DNS 查询处理      | 转发上游        | 上游延迟 + < 1ms |

## 许可证

MIT
