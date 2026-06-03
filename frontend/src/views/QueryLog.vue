<template>
  <div class="p-6">
    <div class="flex items-center justify-between mb-5">
      <h1 class="text-xl font-semibold">查询日志</h1>
      <div class="flex items-center gap-2">
        <span :class="live ? 'text-green-400' : 'text-slate-500'" class="text-xs">
          {{ live ? '● 实时' : '○ 已暂停' }}
        </span>
        <button @click="live = !live" class="btn-ghost text-xs">
          {{ live ? '暂停' : '继续' }}
        </button>
      </div>
    </div>

    <!-- Filters -->
    <div class="card mb-4 flex flex-wrap gap-3 items-center">
      <input v-model="filterDomain" class="input max-w-xs" placeholder="过滤域名…"/>
      <select v-model="filterAction" class="input max-w-[140px]">
        <option value="">全部操作</option>
        <option value="block">拦截</option>
        <option value="allow">放行</option>
        <option value="cache">缓存</option>
        <option value="rewrite">重写</option>
      </select>
      <select v-model="filterQtype" class="input max-w-[100px]">
        <option value="">全部类型</option>
        <option>A</option><option>AAAA</option><option>CNAME</option>
        <option>MX</option><option>TXT</option>
      </select>
      <span class="text-xs text-slate-500 ml-auto">{{ filtered.length }} 条</span>
    </div>

    <!-- Table -->
    <div class="card overflow-hidden p-0">
      <div class="overflow-x-auto">
        <table class="w-full text-sm">
          <thead>
            <tr class="border-b border-slate-800 text-xs text-slate-500">
              <th class="text-left px-4 py-2.5 font-medium">时间</th>
              <th class="text-left px-4 py-2.5 font-medium">域名</th>
              <th class="text-left px-4 py-2.5 font-medium">类型</th>
              <th class="text-left px-4 py-2.5 font-medium">操作</th>
              <th class="text-left px-4 py-2.5 font-medium">客户端</th>
              <th class="text-right px-4 py-2.5 font-medium">延迟</th>
              <th class="px-4 py-2.5"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(q, i) in paged" :key="i"
              class="border-b border-slate-800/60 hover:bg-slate-800/30 transition-colors">
              <td class="px-4 py-2.5 text-slate-500 whitespace-nowrap font-mono text-xs">
                {{ fmtTime(q.ts) }}
              </td>
              <td class="px-4 py-2.5 font-mono max-w-[280px] truncate">
                <span :title="q.domain">{{ q.domain }}</span>
              </td>
              <td class="px-4 py-2.5 text-slate-400">{{ q.qtype }}</td>
              <td class="px-4 py-2.5">
                <span :class="badgeClass(q.action)">{{ q.action }}</span>
              </td>
              <td class="px-4 py-2.5 text-slate-500 font-mono text-xs">{{ q.client }}</td>
              <td class="px-4 py-2.5 text-right text-slate-500 text-xs">{{ q.latency_ms }}ms</td>
              <td class="px-4 py-2.5">
                <button @click="addBlock(q.domain)"
                  v-if="q.action !== 'block'"
                  class="text-xs text-slate-600 hover:text-red-400 transition-colors"
                  title="添加拦截规则">
                  拦截
                </button>
              </td>
            </tr>
            <tr v-if="!paged.length">
              <td colspan="7" class="px-4 py-10 text-center text-slate-600">暂无查询记录</td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      <div class="flex items-center justify-between px-4 py-3 border-t border-slate-800">
        <button @click="page = Math.max(1, page-1)" :disabled="page<=1"
          class="btn-ghost text-xs disabled:opacity-30">上一页</button>
        <span class="text-xs text-slate-500">第 {{ page }} 页 / 共 {{ totalPages }} 页</span>
        <button @click="page = Math.min(totalPages, page+1)" :disabled="page>=totalPages"
          class="btn-ghost text-xs disabled:opacity-30">下一页</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useApiStore } from '../stores/api.js'

const store = useApiStore()
const filterDomain = ref('')
const filterAction = ref('')
const filterQtype  = ref('')
const live = ref(true)
const page = ref(1)
const PAGE_SIZE = 50

const filtered = computed(() => {
  return (store.queries || []).filter(q => {
    if (filterDomain.value && !q.domain.includes(filterDomain.value)) return false
    if (filterAction.value && q.action !== filterAction.value) return false
    if (filterQtype.value  && q.qtype  !== filterQtype.value)  return false
    return true
  })
})

const totalPages = computed(() => Math.max(1, Math.ceil(filtered.value.length / PAGE_SIZE)))
const paged = computed(() => filtered.value.slice((page.value-1)*PAGE_SIZE, page.value*PAGE_SIZE))

watch([filterDomain, filterAction, filterQtype], () => { page.value = 1 })

function fmtTime(ts) {
  const d = new Date(ts)
  return d.toTimeString().slice(0,8)
}

function badgeClass(action) {
  return {
    block:   'badge-block',
    allow:   'badge-allow',
    cache:   'badge-cache',
    rewrite: 'badge-rewrite',
  }[action] || 'badge-allow'
}

async function addBlock(domain) {
  await store.addCustomRule(`||${domain}^`)
  // Reload so engine picks up new rule
  await store.reloadRules()
}

let timer
onMounted(async () => {
  await store.fetchQueries(500)
  timer = setInterval(async () => {
    if (live.value) await store.fetchQueries(500)
  }, 2000)
})
onUnmounted(() => clearInterval(timer))
</script>
