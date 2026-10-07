/** 全局配置仓库：启动时加载一次，页面修改后显式 persist() */
import { computed, ref } from 'vue'
import type * as T from '@/types'
import { getConfig, saveConfig, resetConfig } from '@/api/tauri'

export const config = ref<T.AppConfig | null>(null)
export const loaded = ref(false)
export const loadError = ref('')

/** 预览模式（非 Tauri）下的占位配置，保证 UI 可渲染 */
const placeholder = (): T.AppConfig => ({
  general: {
    workspaceDir: '~/KernelBuilder',
    kernelDir: '',
    initialized: false,
    bootstrapped: [],
  },
  mirror: {
    enabled: true,
    activeId: 'ghproxy',
    autoFallback: true,
    googlesourceMirror: '',
    mirrors: [
      { id: 'direct', label: '直连 GitHub', kind: 'direct', prefix: '', note: '不经过任何代理', enabled: true },
      {
        id: 'ghproxy',
        label: 'ghproxy.net',
        kind: 'prefix',
        prefix: 'https://ghproxy.net/',
        note: '通用 GitHub 代理',
        enabled: true,
      },
      {
        id: 'jsdelivr',
        label: 'jsDelivr CDN',
        kind: 'jsdelivr',
        prefix: 'https://cdn.jsdelivr.net/gh/',
        note: '仅适合 raw 文件',
        enabled: true,
      },
    ],
  },
  source: {
    repo: '',
    branch: 'main',
    protocol: 'auto',
    shallow: true,
    depth: 1,
    singleBranch: true,
    submodules: false,
    sshDetected: false,
  },
  toolchain: {
    installDir: '~/KernelBuilder/toolchains',
    clangBranch: 'main',
    autoDownload: true,
    clangOverride: '',
    gccOverride: '',
  },
  susfs: {
    repo: 'JackA1ltman/NonGKI_Kernel_Build_2nd',
    branch: 'main',
    applyMethod: 'auto',
    keepRejects: true,
    fullClone: true,
  },
  ksu: {
    provider: 'kernelsu',
    branch: 'main',
    method: 'setup',
    downloadManager: true,
    managerPath: '',
  },
  github: {
    token: '',
    owner: '',
    repo: '',
    workflowId: '',
    gitRef: 'main',
    apiBase: 'https://api.github.com',
  },
  build: {
    arch: 'arm64',
    defconfig: '',
    target: 'Image',
    jobs: 0,
    useCcache: true,
    useLto: false,
    outDir: 'out',
    extraArgs: '',
    packageAnykernel: false,
    anykernelRepo: 'osm0sis/AnyKernel3',
  },
  ui: { theme: 'dark', followLog: true, maxLogLines: 5000 },
})

export async function loadConfig() {
  try {
    config.value = await getConfig()
    loaded.value = true
  } catch (e) {
    loadError.value = String(e)
    config.value = placeholder()
    loaded.value = true
  }
  applyTheme()
  return config.value
}

export async function persist(): Promise<void> {
  if (!config.value) return
  try {
    config.value = await saveConfig(config.value)
    loadError.value = ''
  } catch (e) {
    loadError.value = String(e)
  }
}

export async function restoreDefaults() {
  try {
    config.value = await resetConfig()
  } catch {
    config.value = placeholder()
  }
  applyTheme()
}

/* ----------------------------- 主题 ----------------------------- */

export type ThemeMode = 'light' | 'dark' | 'system'

export function applyTheme(mode?: ThemeMode) {
  const m = mode ?? (config.value?.ui.theme as ThemeMode) ?? 'dark'
  const html = document.documentElement
  html.classList.remove('theme-light', 'theme-dark')
  const resolved: 'light' | 'dark' =
    m === 'system'
      ? window.matchMedia?.('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light'
      : m
  html.classList.add(`theme-${resolved}`)
  html.dataset.theme = resolved
  html.style.colorScheme = resolved
  if (config.value && mode) config.value.ui.theme = mode
}

export function setTheme(mode: ThemeMode) {
  applyTheme(mode)
  void persist()
}

/* ----------------------------- 便捷访问 ----------------------------- */

export const kernelDir = computed(() => config.value?.general.kernelDir ?? '')
export const workspaceDir = computed(() => config.value?.general.workspaceDir ?? '')
export const hasKernel = computed(() => !!kernelDir.value)
export const initialized = computed(() => config.value?.general.initialized ?? false)
