<template>
  <div class="log-panel">
    <div class="log-toolbar">
      <button class="tb-btn" @click="clearLogs">清空</button>
      <button class="tb-btn" @click="copyAll">复制</button>
      <button class="tb-btn" @click="download">导出</button>

      <select v-model="taskKey" class="tb-select" title="按模块过滤">
        <option v-for="t in taskKeys" :key="t" :value="t">
          {{ TASK_LABELS[t] ?? t }}
        </option>
      </select>

      <select v-model="levelKey" class="tb-select" title="按等级过滤">
        <option value="all">全部等级</option>
        <option value="error">仅错误</option>
        <option value="warn">警告 + 错误</option>
        <option value="success">仅成功</option>
        <option value="info">仅信息</option>
      </select>

      <input v-model="keyword" class="tb-input" placeholder="过滤关键字…" spellcheck="false" />

      <label class="tb-check">
        <input type="checkbox" v-model="follow" />
        <span>自动滚动</span>
      </label>

      <span class="spacer" />
      <span class="muted mono">{{ visibleLogs.length }}/{{ logs.length }}</span>
      <button v-if="compact" class="tb-btn" @click="emit('close')">收起</button>
    </div>

    <div ref="bodyRef" class="log-body">
      <template v-for="l in visibleLogs" :key="l.seq">
        <div class="log-line" :class="`log-${l.level}`">
          <span class="log-ts">{{ formatTime(l.ts) }}</span>
          <span class="log-task">{{ TASK_LABELS[l.task] ?? l.task }}</span>
          <span class="log-text" v-html="highlight(l.text)"></span>
        </div>
      </template>
      <div v-if="!visibleLogs.length" class="log-empty muted">
        {{ logs.length ? '当前过滤条件下没有日志' : '暂无日志输出' }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import { formatTime } from '@/api/tauri'
import { TASK_LABELS, clearLogs, filter, logs } from '@/stores/log'

const props = defineProps<{ compact?: boolean }>()
const emit = defineEmits<{ close: [] }>()

const bodyRef = ref<HTMLElement | null>(null)
const follow = ref(true)

/* ---- 过滤器（本地状态，直接读写全局 filter，避免对象引用错位） ---- */
const taskKey = ref(filter.value.task)
const levelKey = ref(filter.value.level)
const keyword = ref(filter.value.keyword)

watch(taskKey, (v) => (filter.value.task = v))
watch(levelKey, (v) => (filter.value.level = v))
watch(keyword, (v) => (filter.value.keyword = v))

const taskKeys = computed(() => ['all', ...Object.keys(TASK_LABELS).filter((k) => k !== 'all')])

/** 高亮错误/警告关键字，输入先做 HTML 转义 */
const RULES: { re: RegExp; cls: string }[] = [
  { re: /\b(error|fatal|failed|failure|cannot|not found|undefined reference)\b/gi, cls: 'hl-err' },
  { re: /\b(warn(?:ing)?|deprecated|skipped)\b/gi, cls: 'hl-warn' },
  { re: /\b(success|done|ok|complete[d]?)\b/gi, cls: 'hl-ok' },
]

function escapeHtml(s: string) {
  return s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!)
}

function highlight(text: string) {
  let out = escapeHtml(text)
  for (const r of RULES) {
    out = out.replace(r.re, (m) => `<mark class="${r.cls}">${m}</mark>`)
  }
  return out
}

/* ---- 过滤（在组件内完成，避免与全局 store 状态竞争） ---- */
const visibleLogs = computed(() =>
  logs.value.filter((l) => {
    if (taskKey.value !== 'all' && l.task !== taskKey.value) return false
    const lv = levelKey.value
    if (lv === 'error' && l.level !== 'error') return false
    if (lv === 'warn' && l.level !== 'warn' && l.level !== 'error') return false
    if (lv === 'success' && l.level !== 'success') return false
    if (lv === 'info' && l.level !== 'info') return false
    const kw = keyword.value.trim().toLowerCase()
    if (kw && !l.text.toLowerCase().includes(kw)) return false
    return true
  }),
)

/* ---- 自动滚动 ---- */
watch(
  () => visibleLogs.value.length,
  async () => {
    if (!follow.value) return
    await nextTick()
    const el = bodyRef.value
    if (el) el.scrollTop = el.scrollHeight
  },
)

watch(
  () => props.compact,
  () => {
    void nextTick(() => {
      const el = bodyRef.value
      if (el && follow.value) el.scrollTop = el.scrollHeight
    })
  },
)

/* ---- 导出 ---- */
function textOf() {
  return visibleLogs.value.map((l) => `[${l.task}/${l.level}] ${l.text}`).join('\n')
}

async function copyAll() {
  await navigator.clipboard?.writeText(textOf())
}

function download() {
  const blob = new Blob([textOf()], { type: 'text/plain;charset=utf-8' })
  const a = document.createElement('a')
  a.href = URL.createObjectURL(blob)
  a.download = `kernel-builder-log-${Date.now()}.txt`
  a.click()
  URL.revokeObjectURL(a.href)
}

defineExpose({ scrollToBottom: () => nextTick(() => { const el = bodyRef.value; if (el) el.scrollTop = el.scrollHeight }) })
</script>

<style scoped>
.tb-btn {
  border: 1px solid var(--ctrl-border, #555);
  background: transparent;
  color: var(--text-primary);
  border-radius: 5px;
  padding: 3px 10px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
}

.tb-btn:hover {
  background: var(--ctrl-fill-secondary, rgba(255, 255, 255, 0.08));
}

.tb-select,
.tb-input {
  height: 26px;
  border: 1px solid var(--ctrl-border, #555);
  background: var(--ctrl-fill-default, #2b2b2b);
  color: var(--text-primary);
  border-radius: 5px;
  padding: 0 8px;
  font-size: 12px;
  outline: none;
}

.tb-input {
  width: 200px;
}

.tb-select:focus,
.tb-input:focus {
  border-color: var(--accent-default, #60cdff);
}

.tb-check {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  cursor: pointer;
}

.log-empty {
  padding: 14px;
}

:deep(mark.hl-err) {
  background: var(--SystemFillColorCriticalBackgroundBrush);
  color: var(--SystemFillColorCriticalBrush);
  border-radius: 3px;
  padding: 0 2px;
}

:deep(mark.hl-warn) {
  background: var(--SystemFillColorCautionBackgroundBrush);
  color: var(--SystemFillColorCautionBrush);
  border-radius: 3px;
  padding: 0 2px;
}

:deep(mark.hl-ok) {
  color: var(--SystemFillColorSuccessBrush);
  font-weight: 600;
}
</style>
