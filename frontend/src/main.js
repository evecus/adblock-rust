import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHashHistory } from 'vue-router'
import App from './App.vue'
import './style.css'

import Dashboard from './views/Dashboard.vue'
import QueryLog  from './views/QueryLog.vue'
import Rules     from './views/Rules.vue'
import Settings  from './views/Settings.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/',        component: Dashboard },
    { path: '/queries', component: QueryLog  },
    { path: '/rules',   component: Rules     },
    { path: '/settings',component: Settings  },
  ]
})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
