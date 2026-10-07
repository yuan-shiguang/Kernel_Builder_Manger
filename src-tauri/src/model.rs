//! 前后端共用的数据模型（serde 直出 camelCase，前端 TS 一一对应）

use serde::{Deserialize, Serialize};

/* ============================ 日志 ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub seq: u64,
    /// 任务标识：clone / build / patch / ksu / download / system
    pub task: String,
    /// stdout | stderr | system
    pub stream: String,
    /// info | warn | error | success
    pub level: String,
    pub text: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub id: String,
    pub label: String,
    pub received: u64,
    pub total: u64,
    pub percent: f64,
    pub done: bool,
    pub error: Option<String>,
}

/* ============================ 配置 ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub mirror: MirrorConfig,
    pub source: SourceConfig,
    pub toolchain: ToolchainConfig,
    pub susfs: SusfsConfig,
    pub ksu: KsuConfig,
    pub github: GithubConfig,
    pub build: BuildConfig,
    pub ui: UiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            mirror: MirrorConfig::default(),
            source: SourceConfig::default(),
            toolchain: ToolchainConfig::default(),
            susfs: SusfsConfig::default(),
            ksu: KsuConfig::default(),
            github: GithubConfig::default(),
            build: BuildConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    /// 工作区根目录（源码/工具链/产物的父目录）
    pub workspace_dir: String,
    /// 当前内核源码目录
    pub kernel_dir: String,
    /// 首次初始化是否已完成
    pub initialized: bool,
    /// 初始化时下载的必备组件 id 列表
    pub bootstrapped: Vec<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            workspace_dir: format!("{home}/KernelBuilder"),
            kernel_dir: String::new(),
            initialized: false,
            bootstrapped: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorConfig {
    pub enabled: bool,
    pub active_id: String,
    /// 镜像失败时自动回退直连
    pub auto_fallback: bool,
    pub mirrors: Vec<Mirror>,
    /// android.googlesource.com 的镜像前缀（可留空）
    pub googlesource_mirror: String,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            active_id: "ghproxy".into(),
            auto_fallback: true,
            mirrors: Mirror::builtin(),
            googlesource_mirror: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mirror {
    pub id: String,
    pub label: String,
    /// prefix | jsdelivr | direct
    pub kind: String,
    /// kind=prefix 时为前置域名，kind=direct 时为空
    pub prefix: String,
    pub note: String,
    pub enabled: bool,
}

impl MirrorConfig {
    /// 当前启用的镜像（未开启镜像功能时返回 None）
    pub fn active_mirror(&self) -> Option<&Mirror> {
        if !self.enabled {
            return None;
        }
        self.mirrors.iter().find(|m| m.id == self.active_id && m.enabled)
    }
}

impl Mirror {
    pub fn builtin() -> Vec<Mirror> {
        vec![
            Mirror {
                id: "direct".into(),
                label: "直连 GitHub".into(),
                kind: "direct".into(),
                prefix: String::new(),
                note: "海外网络 / 已挂代理时使用".into(),
                enabled: true,
            },
            Mirror {
                id: "ghproxy".into(),
                label: "ghproxy.net".into(),
                kind: "prefix".into(),
                prefix: "https://ghproxy.net/".into(),
                note: "国内常用，稳定".into(),
                enabled: true,
            },
            Mirror {
                id: "gh-proxy".into(),
                label: "gh-proxy.com".into(),
                kind: "prefix".into(),
                prefix: "https://gh-proxy.com/".into(),
                note: "备用加速".into(),
                enabled: true,
            },
            Mirror {
                id: "ghfast".into(),
                label: "ghfast.top".into(),
                kind: "prefix".into(),
                prefix: "https://ghfast.top/".into(),
                note: "自用加速节点".into(),
                enabled: true,
            },
            Mirror {
                id: "gitmirror".into(),
                label: "hub.gitmirror.com".into(),
                kind: "prefix".into(),
                prefix: "https://hub.gitmirror.com/".into(),
                note: "hub 镜像".into(),
                enabled: true,
            },
            Mirror {
                id: "llkk".into(),
                label: "gh.llkk.cc".into(),
                kind: "prefix".into(),
                prefix: "https://gh.llkk.cc/".into(),
                note: "备用节点".into(),
                enabled: true,
            },
            Mirror {
                id: "jsdelivr".into(),
                label: "jsDelivr CDN".into(),
                kind: "jsdelivr".into(),
                prefix: String::new(),
                note: "仅对 raw / 仓库文件路径生效，Release 资产不适用".into(),
                enabled: true,
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    /// owner/repo 或完整 URL
    pub repo: String,
    pub branch: String,
    /// auto | https | ssh
    pub protocol: String,
    /// 是否使用 --depth=1（设置中可关闭）
    pub shallow: bool,
    pub depth: u32,
    pub single_branch: bool,
    pub submodules: bool,
    /// 检测到的 SSH 状态（只读展示）
    pub ssh_detected: bool,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: "main".into(),
            protocol: "auto".into(),
            shallow: true,
            depth: 1,
            single_branch: true,
            submodules: false,
            ssh_detected: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainConfig {
    pub install_dir: String,
    /// googlesource 上 clang 所在分支
    pub clang_branch: String,
    /// 缺失时是否提示/自动下载
    pub auto_download: bool,
    /// 用户手动覆盖的 clang（留空=自动按内核版本推断）
    pub clang_override: String,
    /// 用户手动覆盖的 GCC aarch64
    pub gcc_override: String,
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            install_dir: format!("{home}/KernelBuilder/toolchains"),
            clang_branch: "main".into(),
            auto_download: true,
            clang_override: String::new(),
            gcc_override: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SusfsConfig {
    pub repo: String,
    pub branch: String,
    /// 补丁应用方式：auto(git apply→patch) | git | patch
    pub apply_method: String,
    /// 失败时是否保留 .rej 以便排查
    pub keep_rejects: bool,
    /// true = 完整 git 克隆（不浅克隆，可自由切换分支）；false = 仅下载 tarball
    pub full_clone: bool,
}

impl Default for SusfsConfig {
    fn default() -> Self {
        Self {
            repo: "JackA1ltman/NonGKI_Kernel_Build_2nd".into(),
            branch: "main".into(),
            apply_method: "auto".into(),
            keep_rejects: true,
            full_clone: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KsuConfig {
    /// provider id
    pub provider: String,
    /// 分支（全部分支由 GitHub API 动态拉取）
    pub branch: String,
    /// setup | manual
    pub method: String,
    /// 是否同时下载管理器 APK
    pub download_manager: bool,
    /// 已保存的管理器 APK 路径
    pub manager_path: String,
}

impl Default for KsuConfig {
    fn default() -> Self {
        Self {
            provider: "kernelsu".into(),
            branch: "main".into(),
            method: "setup".into(),
            download_manager: true,
            manager_path: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GithubConfig {
    pub token: String,
    pub owner: String,
    pub repo: String,
    pub workflow_id: String,
    pub git_ref: String,
    pub api_base: String,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            token: String::new(),
            owner: String::new(),
            repo: String::new(),
            workflow_id: String::new(),
            git_ref: "main".into(),
            api_base: "https://api.github.com".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildConfig {
    pub arch: String,
    pub defconfig: String,
    /// Image | Image.gz | Image.gz-dtb | dtb | all
    pub target: String,
    /// 0 = 自动（nproc）
    pub jobs: u32,
    pub use_ccache: bool,
    pub use_lto: bool,
    pub out_dir: String,
    pub extra_args: String,
    /// 构建完成后是否打包 AnyKernel3
    pub package_anykernel: bool,
    pub anykernel_repo: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            arch: "arm64".into(),
            defconfig: String::new(),
            target: "Image".into(),
            jobs: 0,
            use_ccache: true,
            use_lto: false,
            out_dir: "out".into(),
            extra_args: String::new(),
            package_anykernel: false,
            anykernel_repo: "osm0sis/AnyKernel3".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    /// light | dark | system
    pub theme: String,
    pub follow_log: bool,
    pub max_log_lines: u32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            follow_log: true,
            max_log_lines: 5000,
        }
    }
}

/* ============================ 内核 & 工具链 ============================ */

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KernelInfo {
    pub path: String,
    /// 例如 "4.9"
    pub version: String,
    /// 例如 "4.9.337"
    pub full_version: String,
    pub version_num: u32,
    pub patchlevel_num: u32,
    pub sublevel_num: u32,
    pub has_makefile: bool,
    pub archs: Vec<String>,
    pub defconfigs: Vec<String>,
    /// 是否为源码目录
    pub valid: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainItem {
    pub id: String,
    pub name: String,
    /// clang | gcc
    pub kind: String,
    /// host / aarch64 / arm32
    pub arch: String,
    pub version: String,
    /// 主下载地址（googlesource）
    pub url: String,
    /// GitHub 兜底仓库 owner/repo
    pub github_fallback: String,
    /// 解压后的目录名
    pub dir_name: String,
    pub installed: bool,
    pub path: String,
    pub size_hint: String,
    /// 首次初始化必备
    pub required: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolchainPlan {
    pub kernel_version: String,
    pub matched: bool,
    pub reason: String,
    pub clang: Option<ToolchainItem>,
    pub gcc_aarch64: Option<ToolchainItem>,
    pub gcc_arm32: Option<ToolchainItem>,
    pub missing: Vec<ToolchainItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDependency {
    pub name: String,
    pub present: bool,
    pub required: bool,
    pub note: String,
}

/* ============================ 补丁 ============================ */

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatchFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    /// 从补丁头解析出的目标文件提示
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RejectInfo {
    pub file: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatchApplyResult {
    pub ok: bool,
    pub method: String,
    pub exit_code: i32,
    pub applied: Vec<String>,
    pub failed: Vec<String>,
    pub rejects: Vec<RejectInfo>,
    /// 完整的失败详情（用于 UI 展开）
    pub detail: String,
    pub log: String,
    pub message: String,
}

/* ============================ KSU ============================ */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KsuProvider {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub repo: String,
    pub author: String,
    pub default_branch: String,
    pub description: String,
    /// 管理器 APK 的 release 仓库（默认同 repo）
    pub manager_repo: String,
    pub homepage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitRefInfo {
    pub name: String,
    pub sha: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KsuStatus {
    pub integrated: bool,
    pub provider: String,
    pub version: String,
    pub config_flags: Vec<String>,
    pub files: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseAsset {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub download_url: String,
    pub tag: String,
}

/* ============================ GitHub Actions ============================ */

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GhWorkflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GhRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: String,
    pub html_url: String,
    pub created_at: String,
    pub head_branch: String,
    pub display_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GhArtifact {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub expired: bool,
    pub run_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub ok: bool,
    pub code: i32,
    pub message: String,
    pub output: String,
}

/// 构建产物条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactInfo {
    /// 文件名，如 Image.gz-dtb
    pub name: String,
    /// 绝对路径（供「打开」按钮使用）
    pub path: String,
    /// 字节数
    pub size: u64,
}
