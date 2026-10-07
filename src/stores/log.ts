/** 日志仓库：订阅后端的 log://line 与 progress://download 事件 */
import { computed, ref } from 'vue'
import type * as T from '@/types'
import { onLog, onProgress } from '@/api/tauri'

export const logs = ref<T.LogLine[]>([])
export const downloads = ref<Record<string, T.ProgressEvent>>({})

/** 过滤条件由 LogPanel 直接读写；level 支持 all|error|warn|success|info */
export const filter = ref<{ task: string; level: string; keyword: string }>({
  task: 'all',
  level: 'all',
  keyword: '',
})

export const errors = computed(() => logs.value.filter((l) => l.level === 'error'))
export const errorCount = computed(() => errors.value.length)
export const warnCount = computed(() => logs.value.filter((l) => l.level === 'warn').length)

export const TASK_LABELS: Record<string, string> = {
  all: '全部模块',
  system: '系统',
  init: '初始化',
  clone: '源码克隆',
  kernel: '内核识别',
  toolchain: '工具链',
  susfs: 'SUSFS 补丁',
  ksu: 'KernelSU',
  build: '构建',
  download: '下载',
  actions: '远程编译',
  patch: '补丁',
  config: '配置',
}

const HARD_MAX = 20000
let started = false

/** 当前生效的最大保留行数（跟随 ui.maxLogLines，由 main.ts 在配置加载后同步） */
export const maxLines = ref(5000)

export async function startListeners() {
  if (started) return
  started = true

  await onLog((line) => {
    logs.value.push(line)
    const cap = Math.max(500, Math.min(HARD_MAX, maxLines.value))
    if (logs.value.length > cap) logs.value.splice(0, logs.value.length - cap)
  })

  await onProgress((p) => {
    downloads.value = { ...downloads.value, [p.id]: p }
    if (p.done && p.error) {
      logs.value.push({
        seq: Date.now(),
        task: 'download',
        stream: 'system',
        level: 'error',
        text: `${p.label} 下载失败：${p.error}`,
        ts: Date.now(),
      })
    }
  })
}

export function clearLogs() {
  logs.value = []
}

/** 按当前配置的行数上限裁剪（配置变化时调用） */
export function trimLogs(max: number) {
  maxLines.value = Math.max(500, Math.min(HARD_MAX, max || 5000))
  if (logs.value.length > maxLines.value) {
    logs.value.splice(0, logs.value.length - maxLines.value)
  }
}

export const activeDownloads = computed(() =>
  Object.values(downloads.value).filter((d) => !d.done),
)

/** 导出日志为纯文本 */
export function exportLogs(): string {
  return logs.value.map((l) => `[${l.task}/${l.level}] ${l.text}`).join('\n')
}
