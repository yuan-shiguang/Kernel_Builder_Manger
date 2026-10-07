<template>
  <div class="stat-card">
    <div class="stat-label">{{ label }}</div>
    <div class="stat-value" :class="toneClass">{{ value }}</div>
    <div v-if="hint" class="stat-hint">{{ hint }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  label: string
  value: string
  hint?: string
  tone?: 'ok' | 'warn' | 'err' | 'mute'
}>()

const toneClass = computed(() => `tone-${props.tone ?? 'mute'}`)
</script>

<style scoped>
.stat-card {
  background: var(--card-bg);
  border: 1px solid var(--card-stroke);
  border-radius: var(--app-radius, 8px);
  padding: 14px 16px;
  min-width: 150px;
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.stat-value {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.2;
  word-break: break-all;
}

.stat-hint {
  margin-top: 6px;
  font-size: 11.5px;
  color: var(--text-tertiary);
}

.tone-ok {
  color: var(--SystemFillColorSuccessBrush);
}

.tone-warn {
  color: var(--SystemFillColorCautionBrush);
}

.tone-err {
  color: var(--SystemFillColorCriticalBrush);
}

.tone-mute {
  color: var(--text-primary);
}
</style>
