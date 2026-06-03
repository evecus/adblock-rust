<template>
  <div class="p-6 max-w-2xl">
    <h1 class="text-xl font-semibold mb-6">设置</h1>

    <div v-if="!cfg" class="text-slate-500">加载中…</div>
    <template v-else>
      <!-- DNS -->
      <div class="card mb-5">
        <h2 class="text-sm font-medium text-slate-300 mb-4">DNS 设置</h2>
        <div class="space-y-4">
          <div>
            <label class="block text-xs text-slate-500 mb-1">监听地址</label>
            <input v-model="cfg.dns.bind" class="input"/>
          </div>
          <div>
            <label class="block text-xs text-slate-500 mb-1">拦截模式</label>
            <select v-model="cfg.dns.block_mode" class="input">
              <option value="nxdomain">NXDOMAIN（域名不存在）</option>
              <option value="zeroip">Zero IP（0.0.0.0）</option>
              <option value="refused">REFUSED（拒绝）</option>
            </select>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="block text-xs text-slate-500 mb-1">缓存大小</label>
              <input v-model.number="cfg.dns.cache_size" type="number" class="input"/>
            </div>
            <div>
              <label class="block text-xs text-slate-500 mb-1">拦截 TTL（秒）</label>
              <input v-model.number="cfg.dns.block_ttl" type="number" class="input"/>
            </div>
          </div>
        </div>
      </div>

      <!-- Upstream -->
      <div class="card mb-5">
        <h2 class="text-sm font-medium text-slate-300 mb-4">上游 DNS</h2>
        <div class="space-y-2 mb-3">
          <div v-for="(srv, i) in cfg.upstream.servers" :key="i"
            class="flex gap-2">
            <input v-model="cfg.upstream.servers[i]" class="input font-mono"/>
            <button @click="cfg.upstream.servers.splice(i,1)"
              class="btn-danger flex-shrink-0 px-2.5">✕</button>
          </div>
        </div>
        <button @click="cfg.upstream.servers.push('8.8.8.8:53')"
          class="btn-ghost text-xs">+ 添加上游</button>
        <div class="mt-4">
          <label class="block text-xs text-slate-500 mb-1">超时（毫秒）</label>
          <input v-model.number="cfg.upstream.timeout_ms" type="number" class="input max-w-[160px]"/>
        </div>
        <label class="flex items-center gap-2 mt-3 text-sm cursor-pointer">
          <input type="checkbox" v-model="cfg.upstream.failover" class="rounded"/>
          <span class="text-slate-400">上游失败时自动切换</span>
        </label>
      </div>

      <!-- Web -->
      <div class="card mb-5">
        <h2 class="text-sm font-medium text-slate-300 mb-4">Web 界面</h2>
        <div>
          <label class="block text-xs text-slate-500 mb-1">监听地址</label>
          <input v-model="cfg.web.bind" class="input"/>
        </div>
      </div>

      <!-- Logging -->
      <div class="card mb-5">
        <h2 class="text-sm font-medium text-slate-300 mb-4">日志</h2>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-xs text-slate-500 mb-1">日志级别</label>
            <select v-model="cfg.log.level" class="input">
              <option>error</option>
              <option>warn</option>
              <option>info</option>
              <option>debug</option>
              <option>trace</option>
            </select>
          </div>
          <div>
            <label class="block text-xs text-slate-500 mb-1">查询日志条数</label>
            <input v-model.number="cfg.dns.query_log_size" type="number" class="input"/>
          </div>
        </div>
      </div>

      <!-- Save -->
      <div class="flex items-center gap-3">
        <button @click="save" :disabled="saving" class="btn-primary">
          {{ saving ? '保存中…' : '保存设置' }}
        </button>
        <span v-if="saveMsg" class="text-sm"
          :class="saveOk ? 'text-green-400' : 'text-red-400'">{{ saveMsg }}</span>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useApiStore } from '../stores/api.js'

const store = useApiStore()
const cfg    = ref(null)
const saving = ref(false)
const saveMsg = ref('')
const saveOk  = ref(true)

async function save() {
  saving.value = true
  saveMsg.value = ''
  const res = await store.saveConfig(cfg.value)
  saving.value = false
  if (res.ok) {
    saveMsg.value = '✓ 已保存'
    saveOk.value = true
  } else {
    saveMsg.value = '✗ 保存失败：' + (res.error || '未知错误')
    saveOk.value = false
  }
  setTimeout(() => { saveMsg.value = '' }, 3000)
}

onMounted(async () => {
  await store.fetchConfig()
  cfg.value = JSON.parse(JSON.stringify(store.config))
})
</script>
