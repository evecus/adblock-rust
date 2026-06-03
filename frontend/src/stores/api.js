import { defineStore } from 'pinia'
import { ref, reactive } from 'vue'

const BASE = window.__API_BASE__ || ''

async function api(method, path, body) {
  const opts = { method, headers: { 'Content-Type': 'application/json' } }
  if (body !== undefined) opts.body = JSON.stringify(body)
  const r = await fetch(BASE + path, opts)
  return r.json()
}

export const useApiStore = defineStore('api', () => {
  const stats    = ref(null)
  const queries  = ref([])
  const rulesets = ref([])
  const config   = ref(null)
  const loading  = reactive({})
  const error    = ref(null)

  async function fetchStats() {
    loading.stats = true
    try { stats.value = await api('GET', '/api/stats') }
    catch(e) { error.value = e.message }
    finally { loading.stats = false }
  }

  async function fetchQueries(limit = 200, domain = '') {
    loading.queries = true
    try {
      const qs = domain ? `?limit=${limit}&domain=${encodeURIComponent(domain)}` : `?limit=${limit}`
      queries.value = await api('GET', '/api/queries' + qs)
    } catch(e) { error.value = e.message }
    finally { loading.queries = false }
  }

  async function testDomain(domain) {
    return api('GET', `/api/test?domain=${encodeURIComponent(domain)}`)
  }

  async function fetchRulesets() {
    rulesets.value = await api('GET', '/api/rules/rulesets')
  }

  async function toggleRuleset(name, enabled) {
    await api('PUT', `/api/rules/rulesets/${encodeURIComponent(name)}/toggle`, { enabled })
    await fetchRulesets()
  }

  async function reloadRules() {
    return api('POST', '/api/rules/reload')
  }

  async function fetchConfig() {
    config.value = await api('GET', '/api/config')
  }

  async function saveConfig(cfg) {
    const r = await api('PUT', '/api/config', cfg)
    if (r.ok) config.value = cfg
    return r
  }

  async function fetchCustomRules() {
    return api('GET', '/api/rules/custom')
  }

  async function addCustomRule(rule) {
    return api('POST', '/api/rules/custom', { rule })
  }

  async function removeCustomRule(rule) {
    return api('DELETE', '/api/rules/custom', { rule })
  }

  return {
    stats, queries, rulesets, config, loading, error,
    fetchStats, fetchQueries, testDomain,
    fetchRulesets, toggleRuleset, reloadRules,
    fetchConfig, saveConfig,
    fetchCustomRules, addCustomRule, removeCustomRule,
  }
})
