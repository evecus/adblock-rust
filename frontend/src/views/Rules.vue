<template>
  <div class="p-6">
    <h1 class="text-xl font-semibold mb-6">规则管理</h1>

    <!-- Rulesets -->
    <div class="card mb-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-medium text-slate-300">.ars 规则集</h2>
        <button @click="reload" :disabled="reloading"
          class="btn-primary text-xs flex items-center gap-1.5">
          <span v-if="reloading">重载中…</span>
          <span v-else>重载规则</span>
        </button>
      </div>

      <div v-if="store.rulesets.length === 0"
        class="text-slate-600 text-sm py-4 text-center">
        未配置规则集，请在 config.json 中添加规则集路径
      </div>

      <div v-for="rs in store.rulesets" :key="rs.name"
        class="flex items-center justify-between py-3 border-b border-slate-800 last:border-0">
        <div>
          <p class="text-sm font-medium">{{ rs.name }}</p>
          <p class="text-xs text-slate-500 font-mono mt-0.5">{{ rs.path }}</p>
        </div>
        <label class="relative inline-flex items-center cursor-pointer">
          <input type="checkbox" class="sr-only peer"
            :checked="rs.enabled"
            @change="store.toggleRuleset(rs.name, !rs.enabled)"/>
          <div class="w-9 h-5 bg-slate-700 peer-focus:outline-none rounded-full peer
            peer-checked:after:translate-x-full peer-checked:after:border-white
            after:content-[''] after:absolute after:top-[2px] after:left-[2px]
            after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all
            peer-checked:bg-indigo-600"></div>
        </label>
      </div>
    </div>

    <!-- Custom rules -->
    <div class="card">
      <h2 class="text-sm font-medium text-slate-300 mb-4">自定义规则</h2>
      <p class="text-xs text-slate-500 mb-3">
        支持 AdGuardHome 语法：<code class="bg-slate-800 px-1 rounded">||example.com^</code>（拦截），
        <code class="bg-slate-800 px-1 rounded">@@||safe.com^</code>（白名单），
        <code class="bg-slate-800 px-1 rounded">/regex/</code>
      </p>

      <!-- Add rule input -->
      <div class="flex gap-2 mb-4">
        <input v-model="newRule" @keydown.enter="addRule" class="input font-mono"
          placeholder="||ads.example.com^"/>
        <button @click="addRule" class="btn-primary flex-shrink-0">添加</button>
      </div>

      <!-- Rules list -->
      <div v-if="customRules.length === 0"
        class="text-slate-600 text-sm py-4 text-center">暂无自定义规则</div>

      <div v-for="rule in customRules" :key="rule"
        class="flex items-center justify-between py-2.5 border-b border-slate-800/60 last:border-0">
        <span class="font-mono text-sm" :class="ruleClass(rule)">{{ rule }}</span>
        <button @click="removeRule(rule)" class="text-xs text-slate-600 hover:text-red-400 transition-colors ml-4">
          删除
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useApiStore } from '../stores/api.js'

const store = useApiStore()
const customRules = ref([])
const newRule = ref('')
const reloading = ref(false)

async function loadCustom() {
  customRules.value = await store.fetchCustomRules()
}

async function addRule() {
  const r = newRule.value.trim()
  if (!r) return
  const res = await store.addCustomRule(r)
  if (res.ok) {
    newRule.value = ''
    await loadCustom()
  }
}

async function removeRule(rule) {
  await store.removeCustomRule(rule)
  await loadCustom()
}

async function reload() {
  reloading.value = true
  await store.reloadRules()
  reloading.value = false
}

function ruleClass(rule) {
  if (rule.startsWith('@@')) return 'text-green-400'
  if (rule.startsWith('||')) return 'text-red-400'
  if (rule.startsWith('/'))  return 'text-amber-400'
  return 'text-slate-300'
}

onMounted(async () => {
  await store.fetchRulesets()
  await loadCustom()
})
</script>
