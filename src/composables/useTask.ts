/** 统一的「执行中 / 结果提示」状态机，页面复用 */
import { ref } from 'vue'
import { errText } from '@/api/tauri'

export interface TaskState {
  busy: boolean
  message: string
  severity: 'Informational' | 'Success' | 'Warning' | 'Error'
}

export function useTask() {
  const busy = ref(false)
  const message = ref('')
  const severity = ref<TaskState['severity']>('Informational')

  async function run<T>(fn: () => Promise<T> | T, okMsg?: string): Promise<T | undefined> {
    busy.value = true
    message.value = ''
    severity.value = 'Informational'
    try {
      const r = await fn()
      if (okMsg) {
        message.value = okMsg
        severity.value = 'Success'
      }
      return r ?? undefined
    } catch (e) {
      message.value = errText(e)
      severity.value = 'Error'
      return undefined
    } finally {
      busy.value = false
    }
  }

  function ok(msg: string) {
    message.value = msg
    severity.value = 'Success'
  }

  function fail(msg: string) {
    message.value = msg
    severity.value = 'Error'
  }

  function warn(msg: string) {
    message.value = msg
    severity.value = 'Warning'
  }

  function clear() {
    message.value = ''
    severity.value = 'Informational'
  }

  return { busy, message, severity, run, ok, fail, warn, clear }
}
