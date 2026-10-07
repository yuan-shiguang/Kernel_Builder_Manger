/** 与 Rust model.rs 一一对应的类型定义（camelCase） */

export interface LogLine {
  seq: number
  task: string
  stream: 'stdout' | 'stderr' | 'system'
  level: 'info' | 'warn' | 'error' | 'success'
  text: string
  ts: number
}

export interface ProgressEvent {
  id: string
  label: string
  received: number
  total: number
  percent: number
  done: boolean
  error?: string | null
}

export interface Mirror {
  id: string
  label: string
  kind: 'prefix' | 'jsdelivr' | 'direct'
  prefix: string
  note: string
  enabled: boolean
}

export interface MirrorConfig {
  enabled: boolean
  activeId: string
  autoFallback: boolean
  mirrors: Mirror[]
  googlesourceMirror: string
}

export interface GeneralConfig {
  workspaceDir: string
  kernelDir: string
  initialized: boolean
  bootstrapped: string[]
}

export interface SourceConfig {
  repo: string
  branch: string
  /** auto | https | ssh */
  protocol: string
  shallow: boolean
  depth: number
  singleBranch: boolean
  submodules: boolean
  sshDetected: boolean
}

export interface ToolchainConfig {
  installDir: string
  clangBranch: string
  autoDownload: boolean
  clangOverride: string
  gccOverride: string
}

export interface SusfsConfig {
  repo: string
  branch: string
  applyMethod: string
  keepRejects: boolean
  fullClone: boolean
}

export interface KsuConfig {
  provider: string
  branch: string
  method: string
  downloadManager: boolean
  managerPath: string
}

export interface GithubConfig {
  token: string
  owner: string
  repo: string
  workflowId: string
  gitRef: string
  apiBase: string
}

export interface BuildConfig {
  arch: string
  defconfig: string
  target: string
  jobs: number
  useCcache: boolean
  useLto: boolean
  outDir: string
  extraArgs: string
  packageAnykernel: boolean
  anykernelRepo: string
}

export interface UiConfig {
  theme: 'light' | 'dark' | 'system'
  followLog: boolean
  maxLogLines: number
}

export interface AppConfig {
  general: GeneralConfig
  mirror: MirrorConfig
  source: SourceConfig
  toolchain: ToolchainConfig
  susfs: SusfsConfig
  ksu: KsuConfig
  github: GithubConfig
  build: BuildConfig
  ui: UiConfig
}

export interface KernelInfo {
  path: string
  version: string
  fullVersion: string
  versionNum: number
  patchlevelNum: number
  sublevelNum: number
  hasMakefile: boolean
  archs: string[]
  defconfigs: string[]
  valid: boolean
  message: string
}

export interface ToolchainItem {
  id: string
  name: string
  kind: 'clang' | 'gcc'
  arch: string
  version: string
  url: string
  githubFallback: string
  dirName: string
  installed: boolean
  path: string
  sizeHint: string
  required: boolean
  note: string
}

export interface ToolchainPlan {
  kernelVersion: string
  matched: boolean
  reason: string
  clang: ToolchainItem | null
  gccAarch64: ToolchainItem | null
  gccArm32: ToolchainItem | null
  missing: ToolchainItem[]
}

export interface HostDependency {
  name: string
  present: boolean
  required: boolean
  note: string
}

export interface PatchFileInfo {
  name: string
  path: string
  size: number
  hint: string
}

export interface RejectInfo {
  file: string
  content: string
}

export interface PatchApplyResult {
  ok: boolean
  method: string
  exitCode: number
  applied: string[]
  failed: string[]
  rejects: RejectInfo[]
  detail: string
  log: string
  message: string
}

export interface KsuProvider {
  id: string
  name: string
  owner: string
  repo: string
  author: string
  defaultBranch: string
  description: string
  managerRepo: string
  homepage: string
}

export interface GitRefInfo {
  name: string
  sha: string
  kind: 'branch' | 'tag'
}

export interface KsuStatus {
  integrated: boolean
  provider: string
  version: string
  configFlags: string[]
  files: string[]
  message: string
}

export interface ReleaseAsset {
  id: number
  name: string
  size: number
  downloadUrl: string
  tag: string
}

export interface GhWorkflow {
  id: number
  name: string
  path: string
  state: string
}

export interface GhRun {
  id: number
  name: string
  status: string
  conclusion: string
  htmlUrl: string
  createdAt: string
  headBranch: string
  displayTitle: string
}

export interface GhArtifact {
  id: number
  name: string
  size: number
  expired: boolean
  runId: number
}

export interface SshStatus {
  hasKey: boolean
  keys: string[]
  agentLoaded: boolean
  githubHostConfigured: boolean
  recommend: string
}

/** 构建产物条目 */
export interface ArtifactInfo {
  name: string
  path: string
  size: number
}

export interface CommandResult {
  ok: boolean
  code: number
  message: string
  output: string
}

export interface BootstrapReport {
  workspace: string
  createdDirs: string[]
  missingDeps: string[]
  installHint: string
  downloaded: string[]
  failed: string[]
  ok: boolean
}

export interface NavItem {
  tag: string
  icon: string
  content: string
}
