/**
 * Tauri IPC 封装层
 * ------------------------------------------------------------------
 * 所有命令在此集中定义，页面只依赖本文件，便于统一处理错误与日志。
 */
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type * as T from '@/types'

export const LOG_EVENT = 'log://line'
export const PROGRESS_EVENT = 'progress://download'

/** 浏览器预览模式下的兜底（没有 Tauri 运行时时不崩溃，便于纯前端调试） */
export const inTauri = (): boolean => {
  try {
    return isTauri()
  } catch {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
  }
}

async function call<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!inTauri()) throw new Error(`[预览模式] 无法调用后端命令：${cmd}`)
  return invoke<T>(cmd, args)
}

/* ============================ 配置 ============================ */

export const getConfig = () => call<T.AppConfig>('get_config')
export const saveConfig = (cfg: T.AppConfig) => call<T.AppConfig>('save_config', { cfg })
export const resetConfig = () => call<T.AppConfig>('reset_config')
export const bootstrapNow = (force: boolean) => call<T.BootstrapReport>('bootstrap_now', { force })
export const hostDependencies = () => call<T.HostDependency[]>('host_dependencies')
export const openPath = (path: string) => call<void>('open_path', { path })
export const cancelTask = (task: string) => call<boolean>('cancel', { task })
export const systemInfo = () => call<Record<string, unknown>>('system_info')

/* ============================ 源码 ============================ */

export const detectSsh = () => call<T.SshStatus>('detect_ssh')
export const previewCloneUrl = (repo: string, protocol: string) =>
  call<string>('preview_clone_url', { repo, protocol })

export const cloneKernel = (o: {
  repo: string
  branch: string
  protocol: string
  shallow: boolean
  depth: number
  singleBranch: boolean
  submodules: boolean
  target: string
}) =>
  call<number>('clone_kernel', {
    repo: o.repo,
    branch: o.branch,
    protocol: o.protocol,
    shallow: o.shallow,
    depth: o.depth,
    singleBranch: o.singleBranch,
    submodules: o.submodules,
    target: o.target,
  })

export const scanKernel = (path: string) => call<T.KernelInfo>('scan_kernel', { path })
export const setKernelDir = (path: string) => call<T.KernelInfo>('set_kernel_dir', { path })
export const listDefconfigs = (path: string, arch: string) =>
  call<string[]>('list_defconfigs', { path, arch })
export const readDefconfig = (path: string, arch: string, name: string) =>
  call<string>('read_defconfig', { path, arch, name })
export const writeDefconfig = (
  path: string,
  arch: string,
  name: string,
  content: string,
  alsoSelect: boolean,
) => call<void>('write_defconfig', { path, arch, name, content, alsoSelect })

/* ============================ 工具链 ============================ */

export const toolchainList = () => call<T.ToolchainItem[]>('toolchain_list')
export const toolchainPlan = () => call<T.ToolchainPlan>('toolchain_plan')
export const toolchainInstall = (id: string) => call<string>('toolchain_install', { id })
export const toolchainInstallMissing = () => call<string[]>('toolchain_install_missing')

/* ============================ SUSFS ============================ */

export const susfsFetch = () => call<string>('susfs_fetch')
export const susfsList = () => call<T.PatchFileInfo[]>('susfs_list')
export const susfsApply = (patches: string[], stopOnError: boolean) =>
  call<T.PatchApplyResult[]>('susfs_apply', { patches, stopOnError })

/* ============================ KSU ============================ */

export const ksuProviders = () => call<T.KsuProvider[]>('ksu_providers')
export const ksuBranches = (provider: string, repoOverride = '', includeTags = true) =>
  call<T.GitRefInfo[]>('ksu_branches', { provider, repoOverride, includeTags })
export const ksuFetch = (
  provider: string,
  gitRef: string,
  ownerOverride = '',
  repoOverride = '',
) => call<string>('ksu_fetch', { provider, gitRef, ownerOverride, repoOverride })
export const ksuIntegrate = (
  provider: string,
  gitRef: string,
  method: string,
  ownerOverride = '',
  repoOverride = '',
) =>
  call<string>('ksu_integrate', { provider, gitRef, method, ownerOverride, repoOverride })
export const ksuStatus = () => call<T.KsuStatus>('ksu_status')
export const ksuManagerAssets = (repo: string) => call<T.ReleaseAsset[]>('ksu_manager_assets', { repo })
export const ksuDownloadManager = (asset: T.ReleaseAsset) =>
  call<string>('ksu_download_manager', { asset })

/* ============================ 构建 ============================ */

export const buildPreview = () => call<string>('build_preview')
export const buildStart = () => call<number>('build_start')
export const buildClean = () => call<void>('build_clean')
export const buildOutputPath = () => call<string>('build_output_path')
export const buildArtifacts = () => call<T.ArtifactInfo[]>('build_artifacts')
export const buildPackage = () => call<string>('build_package')

/* ============================ GitHub Actions ============================ */

export const ghListWorkflows = () => call<T.GhWorkflow[]>('gh_list_workflows')
export const ghDispatch = (workflowId: string, gitRef: string, inputs: Record<string, unknown>) =>
  call<string>('gh_dispatch', { workflowId, gitRef, inputs })
export const ghListRuns = () => call<T.GhRun[]>('gh_list_runs')
export const ghListArtifacts = (runId: number) => call<T.GhArtifact[]>('gh_list_artifacts', { runId })
export const ghDownloadArtifact = (artifact: T.GhArtifact) =>
  call<string>('gh_download_artifact', { artifact })

export const ghCancelRun = (runId: number) =>
  call<T.CommandResult>('gh_cancel_run', { runId })

export const testMirror = (id: string) =>
  call<T.CommandResult>('test_mirror', { id })

/* ============================ 事件 ============================ */

export function onLog(cb: (line: T.LogLine) => void): Promise<UnlistenFn> {
  return listen<T.LogLine>(LOG_EVENT, (e) => cb(e.payload))
}

export function onProgress(cb: (p: T.ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<T.ProgressEvent>(PROGRESS_EVENT, (e) => cb(e.payload))
}

/* ============================ 工具 ============================ */

export function formatSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = bytes
  let i = 0
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024
    i++
  }
  return `${size.toFixed(1)} ${units[i]}`
}

export function formatTime(ts: number): string {
  const d = new Date(ts)
  const p = (n: number, w = 2) => String(n).padStart(w, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

export function errText(e: unknown): string {
  if (typeof e === 'string') return e
  if (e instanceof Error) return e.message
  try {
    return JSON.stringify(e)
  } catch {
    return String(e)
  }
}
