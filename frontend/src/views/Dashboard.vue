<template>
  <div class="p-6">
    <h1 class="text-xl font-semibold mb-6">仪表盘</h1>

    <!-- Stat cards -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
      <div class="card">
        <p class="text-xs text-slate-500 mb-1">总查询</p>
        <p class="text-2xl font-bold text-white">{{ fmt(stats?.queries?.total) }}</p>
      </div>
      <div class="card">
        <p class="text-xs text-slate-500 mb-1">已拦截</p>
        <p class="text-2xl font-bold text-red-400">{{ fmt(stats?.queries?.blocked) }}</p>
        <p class="text-xs text-slate-600 mt-0.5">{{ blockPct }}%</p>
      </div>
      <div class="card">
        <p class="text-xs text-slate-500 mb-1">缓存命中</p>
        <p class="text-2xl font-bold text-blue-400">{{ fmt(stats?.queries?.cached) }}</p>
      </div>
      <div class="card">
        <p class="text-xs text-slate-500 mb-1">规则数</p>
        <p class="text-2xl font-bold text-indigo-400">{{ fmt(stats?.engine?.rule_counts?.total) }}</p>
      </div>
    </div>

    <!-- Charts row -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mb-6">
      <!-- Doughnut: query breakdown -->
      <div class="card flex flex-col">
        <h2 class="text-sm font-medium text-slate-400 mb-4">查询分布</h2>
        <div class="flex-1 flex items-center justify-center" style="height:200px">
          <Doughnut v-if="doughnutData" :data="doughnutData" :options="doughnutOpts"/>
          <span v-else class="text-slate-600 text-sm">暂无数据</span>
        </div>
      </div>

      <!-- Bar: recent history from log -->
      <div class="card flex flex-col">
        <h2 class="text-sm font-medium text-slate-400 mb-4">近期查询趋势（最近100条）</h2>
        <div class="flex-1" style="height:200px">
          <Bar v-if="barData" :data="barData" :options="barOpts"/>
          <span v-else class="text-slate-600 text-sm">暂无数据</span>
        </div>
      </div>
    </div>

    <!-- Engine info -->
    <div class="card mb-6" v-if="stats?.engine">
      <h2 class="text-sm font-medium text-slate-400 mb-3">规则引擎</h2>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
        <div v-for="item in engineRows" :key="item.label">
          <span class="text-slate-500">{{ item.label }}</span>
          <span class="ml-2 text-slate-200">{{ item.value }}</span>
        </div>
      </div>
    </div>

    <!-- Domain test -->
    <div class="card">
      <h2 class="text-sm font-medium text-slate-400 mb-3">域名测试</h2>
      <div class="flex gap-2">
        <input v-model="testInput" @keydown.enter="runTest" class="input"
          placeholder="输入域名，如 ads.example.com"/>
        <button @click="runTest" class="btn-primary flex-shrink-0">测试</button>
      </div>
      <div v-if="testResult" class="mt-3 p-3 rounded-lg text-sm"
        :class="testResultClass">
        <span class="font-mono">{{ testResult.domain }}</span>
        <span class="mx-2">→</span>
        <span class="font-semibold">{{ testResult.result }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { Doughnut, Bar } from 'vue-chartjs'
import {
  Chart as ChartJS, ArcElement, Tooltip, Legend,
  BarElement, CategoryScale, LinearScale
} from 'chart.js'
import { useApiStore } from '../stores/api.js'

ChartJS.register(ArcElement, Tooltip, Legend, BarElement, CategoryScale, LinearScale)

const store = useApiStore()
const stats = computed(() => store.stats)

const testInput  = ref('')
const testResult = ref(null)

const blockPct = computed(() => {
  const t = stats.value?.queries?.total || 0
  const b = stats.value?.queries?.blocked || 0
  return t ? ((b / t) * 100).toFixed(1) : '0.0'
})

const doughnutData = computed(() => {
  const q = stats.value?.queries
  if (!q) return null
  const allowed = (q.total || 0) - (q.blocked || 0) - (q.cached || 0)
  return {
    labels: ['已放行', '已拦截', '缓存命中'],
    datasets: [{
      data: [Math.max(0, allowed), q.blocked || 0, q.cached || 0],
      backgroundColor: ['#22c55e40','#ef444450','#3b82f650'],
      borderColor:     ['#22c55e',  '#ef4444',   '#3b82f6'],
      borderWidth: 1,
    }]
  }
})

const doughnutOpts = {
  responsive: true, maintainAspectRatio: false,
  plugins: { legend: { labels: { color: '#94a3b8', boxWidth: 12 } } }
}

const barData = computed(() => {
  if (!store.queries.length) return null
  // Bucket last 100 queries into 10 time buckets
  const items = store.queries.slice(0, 100).reverse()
  const buckets = Array.from({ length: 10 }, () => ({ allow: 0, block: 0 }))
  items.forEach((q, i) => {
    const b = Math.floor((i / items.length) * 10)
    const bucket = buckets[Math.min(b, 9)]
    if (q.action === 'block') bucket.block++
    else bucket.allow++
  })
  return {
    labels: buckets.map((_, i) => ''),
    datasets: [
      { label: '放行', data: buckets.map(b => b.allow), backgroundColor: '#22c55e40', borderColor: '#22c55e', borderWidth: 1 },
      { label: '拦截', data: buckets.map(b => b.block), backgroundColor: '#ef444440', borderColor: '#ef4444', borderWidth: 1 },
    ]
  }
})

const barOpts = {
  responsive: true, maintainAspectRatio: false,
  scales: {
    x: { stacked: true, ticks: { color: '#475569' }, grid: { color: '#1e293b' } },
    y: { stacked: true, ticks: { color: '#475569' }, grid: { color: '#1e293b' } },
  },
  plugins: { legend: { labels: { color: '#94a3b8', boxWidth: 12 } } }
}

const engineRows = computed(() => {
  const c = stats.value?.engine?.rule_counts
  if (!c) return []
  return [
    { label: '精确拦截', value: c.block_exact  || 0 },
    { label: '后缀拦截', value: c.block_suffix  || 0 },
    { label: '关键词',   value: c.block_keyword || 0 },
    { label: '正则',     value: c.block_regex   || 0 },
    { label: '精确白名单', value: c.allow_exact  || 0 },
    { label: '后缀白名单', value: c.allow_suffix  || 0 },
    { label: '重写规则',  value: c.rewrite       || 0 },
    { label: '缓存条目',  value: stats.value?.cache?.size || 0 },
  ]
})

const fmt = v => (v == null ? '–' : Number(v).toLocaleString())

async function runTest() {
  if (!testInput.value.trim()) return
  testResult.value = await store.testDomain(testInput.value.trim())
}

const testResultClass = computed(() => {
  if (!testResult.value) return ''
  const r = testResult.value.result
  if (r === 'block')        return 'bg-red-900/30 text-red-300'
  if (r === 'allow')        return 'bg-green-900/30 text-green-300'
  if (r?.startsWith('rewrite')) return 'bg-amber-900/30 text-amber-300'
  return 'bg-slate-800 text-slate-300'
})

let timer
onMounted(async () => {
  await store.fetchStats()
  await store.fetchQueries(100)
  timer = setInterval(async () => {
    await store.fetchStats()
    await store.fetchQueries(100)
  }, 3000)
})
onUnmounted(() => clearInterval(timer))
</script>
