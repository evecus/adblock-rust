<template>
  <div class="flex h-screen overflow-hidden">
    <!-- Sidebar -->
    <aside class="w-56 flex-shrink-0 bg-slate-900 border-r border-slate-800 flex flex-col">
      <!-- Logo -->
      <div class="px-5 py-4 border-b border-slate-800">
        <div class="flex items-center gap-2.5">
          <div class="w-7 h-7 rounded-lg bg-indigo-600 flex items-center justify-center">
            <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955
                   11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824
                   10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
            </svg>
          </div>
          <span class="font-semibold text-white">DNS Filter</span>
        </div>
      </div>

      <!-- Nav -->
      <nav class="flex-1 px-3 py-4 space-y-0.5">
        <RouterLink v-for="item in navItems" :key="item.to" :to="item.to"
          class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors"
          :class="$route.path === item.to
            ? 'bg-indigo-600/20 text-indigo-400'
            : 'text-slate-400 hover:text-slate-100 hover:bg-slate-800'">
          <component :is="item.icon" class="w-4 h-4 flex-shrink-0"/>
          {{ item.label }}
        </RouterLink>
      </nav>

      <!-- Status pill -->
      <div class="px-4 py-3 border-t border-slate-800">
        <div class="flex items-center gap-2 text-xs text-slate-500">
          <span class="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse"></span>
          DNS服务运行中
        </div>
      </div>
    </aside>

    <!-- Main content -->
    <main class="flex-1 overflow-auto">
      <RouterView/>
    </main>
  </div>
</template>

<script setup>
import { RouterLink, RouterView, useRoute } from 'vue-router'
import { h } from 'vue'

const $route = useRoute()

// Inline SVG icon components
const IconDash    = { render: () => h('svg', { fill:'none', stroke:'currentColor', viewBox:'0 0 24 24' },
  [h('path', { 'stroke-linecap':'round','stroke-linejoin':'round','stroke-width':'2',
    d:'M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z'
  })])}
const IconList    = { render: () => h('svg', { fill:'none', stroke:'currentColor', viewBox:'0 0 24 24' },
  [h('path', { 'stroke-linecap':'round','stroke-linejoin':'round','stroke-width':'2',
    d:'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2'
  })])}
const IconShield  = { render: () => h('svg', { fill:'none', stroke:'currentColor', viewBox:'0 0 24 24' },
  [h('path', { 'stroke-linecap':'round','stroke-linejoin':'round','stroke-width':'2',
    d:'M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z'
  })])}
const IconCog     = { render: () => h('svg', { fill:'none', stroke:'currentColor', viewBox:'0 0 24 24' },
  [h('path', { 'stroke-linecap':'round','stroke-linejoin':'round','stroke-width':'2',
    d:'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z'
  })])}

const navItems = [
  { to: '/',         label: '仪表盘', icon: IconDash   },
  { to: '/queries',  label: '查询日志', icon: IconList   },
  { to: '/rules',    label: '规则管理', icon: IconShield },
  { to: '/settings', label: '设置',   icon: IconCog    },
]
</script>
