<template>
  <div class="app-root">
    <!-- 顶部自绘标题栏（拖拽区由 WinUIonWeb 的 TitleBarDragRegion 处理） -->
    <div class="app-titlebar" data-tauri-drag-region>
      <span class="tb-glyph"></span>
      <span class="tb-title">Kernel Builder Manager</span>
      <span class="spacer" />
      <span class="tb-chip" :class="inTauriMode ? 'ok' : 'warn'">
        {{ inTauriMode ? 'Tauri 运行时' : '浏览器预览' }}
      </span>
      <span v-if="kernelVersion" class="tb-chip mono">Linux {{ kernelVersion }}</span>
    </div>

    <NavigationView
      class="app-nav"
      :MenuItems="navMenuItems"
      :SelectedItem="selectedItem"
      PaneDisplayMode="Left"
      :IsBackButtonVisible="'Collapsed'"
      :IsSettingsVisible="false"
      :IsPaneToggleButtonVisible="true"
      :IsPaneOpen="paneOpen"
      :OpenPaneLength="228"
      :CompactPaneLength="48"
      PaneTitle="构建管理器"
      @ItemInvoked="onInvoked"
      @update:IsPaneOpen="(v) => (paneOpen = v)"
    >
      <div class="app-content">
        <router-view v-slot="{ Component }">
          <component :is="Component" />
        </router-view>
      </div>
    </NavigationView>

    <!-- 底部日志停靠区 -->
    <transition name="dock">
      <div v-if="logOpen" class="log-dock">
        <LogPanel compact @close="logOpen = false" />
      </div>
    </transition>

    <!-- 状态栏 -->
    <div class="status-bar">
      <span class="badge" :class="hasKernel ? 'badge-ok' : 'badge-mute'">
        {{ hasKernel ? '内核已就绪' : '未选择内核' }}
      </span>
      <span class="muted mono ellipsis" style="max-width: 40%">{{ kernelDir || '—' }}</span>
      <span class="spacer" />

      <span v-for="d in activeDownloads" :key="d.id" class="dl-item">
        <span class="muted">{{ d.label }}</span>
        <span class="dl-bar"><i :style="{ width: `${d.percent}%` }"></i></span>
        <span class="mono">{{ d.percent.toFixed(0) }}%</span>
      </span>

      <span v-if="errorCount" class="badge badge-err">错误 {{ errorCount }}</span>
      <span v-if="warnCount" class="badge badge-warn">警告 {{ warnCount }}</span>

      <button class="status-btn" @click="logOpen = !logOpen">
        {{ logOpen ? '收起日志' : `展开日志（${logCount}）` }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import LogPanel from '@/components/LogPanel.vue'
import { navMenuItems } from '@/router'
import { inTauri } from '@/api/tauri'
import { config, hasKernel, kernelDir, loadConfig } from '@/stores/settings'
import { activeDownloads, errorCount, logs, warnCount } from '@/stores/log'
import { scanKernel } from '@/api/tauri'

const router = useRouter()
const route = useRoute()
const logOpen = ref(false)
const paneOpen = ref(true)
const inTauriMode = inTauri()
const kernelVersion = ref('')

const logCount = computed(() => logs.value.length)

/** NavigationView 的 SelectedItem 用 Tag 匹配（menu item 的 Tag = 路由 name） */
const selectedItem = computed(() => String(route.name ?? 'dashboard'))

interface ItemInvokedArgs {
  InvokedItemContainer?: { Tag?: string; Content?: string }
  IsSettingsInvoked?: boolean
}

function onInvoked(args: ItemInvokedArgs) {
  if (args?.IsSettingsInvoked) {
    void router.push({ name: 'settings' })
    return
  }
  const tag = args?.InvokedItemContainer?.Tag
  if (tag && tag !== String(route.name)) void router.push({ name: tag })
}

/* ---- 主题：随配置实时生效 ---- */
function syncTheme() {
  const mode = config.value?.ui.theme ?? 'dark'
  const html = document.documentElement
  const resolved =
    mode === 'system'
      ? window.matchMedia?.('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
      : mode
  html.classList.remove('theme-light', 'theme-dark')
  html.classList.add(`theme-${resolved}`)
  html.style.colorScheme = resolved
}

watch(() => config.value?.ui.theme, syncTheme)
watch(kernelDir, async (dir) => {
  if (!dir) {
    kernelVersion.value = ''
    return
  }
  try {
    const info = await scanKernel(dir)
    kernelVersion.value = info.valid ? info.fullVersion : ''
  } catch {
    kernelVersion.value = ''
  }
})

onMounted(async () => {
  if (!config.value) await loadConfig()
  syncTheme()
})
</script>

<style scoped>
.app-titlebar {
  flex: 0 0 36px;
  height: 36px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px;
  border-bottom: 1px solid var(--stroke-divider);
  background: var(--SolidBackgroundFillColorTertiaryBrush);
  font-size: 12px;
  user-select: none;
}

.tb-glyph {
  font-family: 'SegoeIcons', 'Segoe Fluent Icons', 'Segoe MDL2 Assets';
  font-size: 14px;
  color: var(--accent-default, #60cdff);
}

.tb-title {
  font-weight: 600;
}

.tb-chip {
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 9px;
  background: var(--subtle-secondary);
  color: var(--text-secondary);
}

.tb-chip.ok {
  background: var(--SystemFillColorSuccessBackgroundBrush);
  color: var(--SystemFillColorSuccessBrush);
}

.tb-chip.warn {
  background: var(--SystemFillColorCautionBackgroundBrush);
  color: var(--SystemFillColorCautionBrush);
}

.spacer {
  flex: 1;
}

.app-nav {
  flex: 1;
  min-height: 0;
}

.app-root {
  height: 100vh;
  overflow: hidden;
}

.log-dock {
  height: 280px;
  flex: 0 0 280px;
  padding: 0 12px 8px;
  box-sizing: border-box;
}

.dock-enter-active,
.dock-leave-active {
  transition: height 0.18s ease, flex-basis 0.18s ease;
}

.dock-enter-from,
.dock-leave-to {
  height: 0;
  flex-basis: 0;
  padding-bottom: 0;
}

.status-bar {
  flex: 0 0 34px;
  height: 34px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 14px;
  border-top: 1px solid var(--stroke-divider);
  background: var(--SolidBackgroundFillColorTertiaryBrush);
  font-size: 12px;
}

.dl-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.dl-bar {
  display: inline-block;
  width: 90px;
  height: 4px;
  border-radius: 2px;
  background: var(--subtle-secondary);
  overflow: hidden;
}

.dl-bar i {
  display: block;
  height: 100%;
  background: var(--accent-default, #60cdff);
}

.status-btn {
  border: 1px solid var(--ctrl-border, #555);
  background: var(--ctrl-fill-default, transparent);
  color: var(--text-primary);
  border-radius: 5px;
  padding: 3px 10px;
  font-size: 12px;
  cursor: pointer;
}

.status-btn:hover {
  background: var(--ctrl-fill-secondary, rgba(255, 255, 255, 0.06));
}

.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
