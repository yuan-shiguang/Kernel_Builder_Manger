import { createApp } from 'vue'
import { createRouter, createWebHashHistory } from 'vue-router'

import App from './App.vue'
import { registerWinUI } from './winuionweb'
import { routes } from './router'
import { loadConfig } from './stores/settings'
import { startListeners, trimLogs } from './stores/log'
import './styles/app.css'

const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

const app = createApp(App)

// WinUIonWeb 控件必须在使用任何 <Button> / <TextBlock> 之前完成注册
registerWinUI(app)

app.use(router)

// 全局错误兜底：避免单个页面异常导致白屏
app.config.errorHandler = (err, _inst, info) => {
  console.error('[vue error]', info, err)
  const el = document.getElementById('app-boot-error')
  if (el) el.textContent = String(err)
}

async function boot() {
  // 1. 先订阅事件总线，确保初始化阶段的日志不丢失
  try {
    await startListeners()
  } catch (e) {
    console.warn('事件订阅失败（浏览器预览模式属正常）', e)
  }

  // 2. 加载配置；失败时使用占位配置，保证 UI 可渲染
  const cfg = await loadConfig()
  if (cfg?.ui?.maxLogLines) trimLogs(cfg.ui.maxLogLines)

  app.mount('#app')
}

void boot()
