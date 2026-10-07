//! Tauri 命令层：前端唯一入口

use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use tauri::{AppHandle, Manager, State};

use crate::bootstrap::{self, BootstrapReport};
use crate::config;
use crate::git;
use crate::github;
use crate::kernel;
use crate::ksu;
use crate::log::{log_error, log_info, log_success};
use crate::mirror;
use crate::model::*;
use crate::proc::cancel_task;
use crate::state::AppState;
use crate::susfs;
use crate::toolchain;

type R<T> = Result<T, String>;

/* ============================ 配置 ============================ */

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> AppConfig {
    state.config_snapshot()
}

#[tauri::command]
pub fn save_config(app: AppHandle, cfg: AppConfig) -> R<AppConfig> {
    config::save(&app, &cfg)?;
    log_info(&app, "config", "配置已保存");
    Ok(cfg)
}

#[tauri::command]
pub fn reset_config(app: AppHandle) -> R<AppConfig> {
    let cfg = AppConfig::default();
    config::save(&app, &cfg)?;
    Ok(cfg)
}

#[tauri::command]
pub async fn bootstrap_now(app: AppHandle, force: bool) -> BootstrapReport {
    let cfg = app.state::<AppState>().config_snapshot();
    bootstrap::run(&app, &cfg, force).await
}

#[tauri::command]
pub fn host_dependencies() -> Vec<HostDependency> {
    toolchain::host_dependencies()
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> R<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在：{path}"));
    }
    // 优先用 xdg-open 打开目录/文件
    let opened = std::process::Command::new("xdg-open")
        .arg(&p)
        .spawn()
        .is_ok();
    if !opened {
        log_error(&app, "system", "xdg-open 不可用，请在文件管理器中手动打开");
        return Err("无法调用 xdg-open".into());
    }
    Ok(())
}

#[tauri::command]
pub fn cancel(task: String, app: AppHandle) -> R<bool> {
    Ok(cancel_task(&app, &task))
}

/* ============================ 源码 ============================ */

#[tauri::command]
pub fn detect_ssh() -> git::SshStatus {
    git::detect_ssh()
}

#[tauri::command]
pub fn preview_clone_url(repo: String, protocol: String) -> R<String> {
    let ssh = git::detect_ssh();
    git::build_clone_url(&repo, &protocol, &ssh)
}

#[tauri::command]
pub fn clone_kernel(
    app: AppHandle,
    repo: String,
    branch: String,
    protocol: String,
    shallow: bool,
    depth: u32,
    single_branch: bool,
    submodules: bool,
    target: String,
) -> R<i32> {
    let dest = PathBuf::from(&target);
    let (code, _) = git::clone(
        &app,
        &repo,
        &branch,
        &protocol,
        shallow,
        depth,
        single_branch,
        submodules,
        &dest,
    )?;
    if code != 0 {
        return Err(format!(
            "克隆失败（退出码 {code}）。可能原因：\n\
             1. 网络不可达 —— 可在设置页切换镜像源\n\
             2. SSH Key 未添加到 GitHub 或未加入 ssh-agent\n\
             3. 分支名不存在\n\
             4. 目标目录非空\n\
             详见日志面板"
        ));
    }
    // 记住内核目录
    let _ = config::update(&app, |c| c.general.kernel_dir = target.clone());
    Ok(code)
}

#[tauri::command]
pub fn scan_kernel(app: AppHandle, path: String) -> KernelInfo {
    let info = kernel::detect(Path::new(&path));
    if info.valid {
        let _ = config::update(&app, |c| c.general.kernel_dir = path.clone());
        log_success(
            &app,
            "kernel",
            &format!("识别到 Linux {}（{} 个 defconfig）", info.full_version, info.defconfigs.len()),
        );
    } else {
        log_error(&app, "kernel", &info.message);
    }
    info
}

#[tauri::command]
pub fn list_defconfigs(path: String, arch: String) -> Vec<String> {
    kernel::list_defconfigs(Path::new(&path), &arch)
}

#[tauri::command]
pub fn read_defconfig(path: String, arch: String, name: String) -> R<String> {
    kernel::read_defconfig(Path::new(&path), &arch, &name)
}

#[tauri::command]
pub fn write_defconfig(
    app: AppHandle,
    path: String,
    arch: String,
    name: String,
    content: String,
    also_select: bool,
) -> R<()> {
    kernel::write_defconfig(Path::new(&path), &arch, &name, &content)?;
    if also_select {
        let _ = config::update(&app, |c| c.build.defconfig = name.clone());
    }
    log_success(&app, "config", &format!("已保存 {name}（原文件备份为 .bak）"));
    Ok(())
}

/* ============================ 工具链 ============================ */

#[tauri::command]
pub fn toolchain_list(state: State<'_, AppState>) -> Vec<ToolchainItem> {
    let cfg = state.config_snapshot();
    toolchain::list_all(&cfg)
}

#[tauri::command]
pub fn toolchain_plan(app: AppHandle, state: State<'_, AppState>) -> ToolchainPlan {
    let cfg = state.config_snapshot();
    let version = if cfg.general.kernel_dir.is_empty() {
        String::new()
    } else {
        kernel::detect(Path::new(&cfg.general.kernel_dir)).version
    };
    let plan = toolchain::plan(&cfg, &version);
    log_info(&app, "toolchain", &plan.reason);
    if !plan.missing.is_empty() {
        log_info(
            &app,
            "toolchain",
            &format!(
                "缺失 {} 个组件：{}",
                plan.missing.len(),
                plan.missing.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join("、")
            ),
        );
    }
    plan
}

#[tauri::command]
pub async fn toolchain_install(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> R<String> {
    let cfg = state.config_snapshot();
    let item = toolchain::list_all(&cfg)
        .into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| format!("未找到工具链：{id}"))?;

    toolchain::install(&app, &cfg, &item).await
}

#[tauri::command]
pub async fn toolchain_install_missing(
    app: AppHandle,
    state: State<'_, AppState>,
) -> R<Vec<String>> {
    let cfg = state.config_snapshot();
    let version = if cfg.general.kernel_dir.is_empty() {
        String::new()
    } else {
        kernel::detect(Path::new(&cfg.general.kernel_dir)).version
    };
    let plan = toolchain::plan(&cfg, &version);
    let mut ok = Vec::new();
    for item in &plan.missing {
        match toolchain::install(&app, &cfg, item).await {
            Ok(p) => ok.push(p),
            Err(e) => log_error(&app, "toolchain", &e),
        }
    }
    Ok(ok)
}

/* ============================ SUSFS ============================ */

#[tauri::command]
pub async fn susfs_fetch(app: AppHandle, state: State<'_, AppState>) -> R<String> {
    let cfg = state.config_snapshot();
    let dir = susfs::fetch(&app, &cfg).await?;
    Ok(dir)
}

#[tauri::command]
pub fn susfs_list(state: State<'_, AppState>) -> Vec<PatchFileInfo> {
    let cfg = state.config_snapshot();
    susfs::list_patches(&cfg)
}

#[tauri::command]
pub fn susfs_apply(
    app: AppHandle,
    state: State<'_, AppState>,
    patches: Vec<String>,
    stop_on_error: bool,
) -> R<Vec<PatchApplyResult>> {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return Err("请先在「源码」页选择内核目录".into());
    }
    let kernel_dir = Path::new(&cfg.general.kernel_dir);
    let paths: Vec<PathBuf> = patches.iter().map(PathBuf::from).collect();
    let results = susfs::apply_many(
        &app,
        kernel_dir,
        &paths,
        &cfg.susfs.apply_method,
        cfg.susfs.keep_rejects,
        stop_on_error,
    );

    // 完整结果（含失败详情）一律返回，前端据此渲染「失败详情」面板；
    // 同时把失败摘要写进日志，便于用户回溯。
    let failed: Vec<&PatchApplyResult> = results.iter().filter(|r| !r.ok).collect();
    if !failed.is_empty() {
        log_error(
            &app,
            "susfs",
            &format!(
                "{} 个补丁应用失败：\n{}",
                failed.len(),
                failed.iter().map(|r| r.message.clone()).collect::<Vec<_>>().join("\n")
            ),
        );
    }
    Ok(results)
}

/* ============================ KSU ============================ */

#[tauri::command]
pub fn ksu_providers() -> Vec<KsuProvider> {
    ksu::providers()
}

#[tauri::command]
pub async fn ksu_branches(
    state: State<'_, AppState>,
    provider: String,
    repo_override: String,
    include_tags: bool,
) -> R<Vec<GitRefInfo>> {
    let cfg = state.config_snapshot();
    let repo = if repo_override.is_empty() {
        ksu::find_provider(&provider)
            .map(|p| format!("{}/{}", p.owner, p.repo))
            .ok_or_else(|| format!("未知 provider：{provider}"))?
    } else {
        repo_override
    };

    // list_branches / list_tags 需要 owner、repo 分开传
    let (owner, repo_name) = mirror::normalize_repo(&repo)
        .ok_or_else(|| format!("仓库地址无效（应为 owner/repo）：{repo}"))?;
    let mut out =
        ksu::list_branches(&owner, &repo_name, &cfg.github.token, &cfg.github.api_base).await?;
    if include_tags {
        if let Ok(tags) =
            ksu::list_tags(&owner, &repo_name, &cfg.github.token, &cfg.github.api_base).await
        {
            out.extend(tags);
        }
    }
    Ok(out)
}

#[tauri::command]
pub async fn ksu_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
    git_ref: String,
    owner_override: String,
    repo_override: String,
) -> R<String> {
    let cfg = state.config_snapshot();
    let mut p = ksu::find_provider(&provider)
        .cloned()
        .ok_or_else(|| format!("未知 provider：{provider}"))?;
    if !owner_override.is_empty() {
        p.owner = owner_override;
    }
    if !repo_override.is_empty() {
        p.repo = repo_override;
    }
    let dir = ksu::fetch_source(&app, &cfg, &p.owner, &p.repo, &p.name, &git_ref).await?;
    Ok(dir.display().to_string())
}

#[tauri::command]
pub async fn ksu_integrate(
    app: AppHandle,
    state: State<'_, AppState>,
    provider: String,
    git_ref: String,
    method: String,
    owner_override: String,
    repo_override: String,
) -> R<String> {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return Err("请先在「源码」页选择内核目录".into());
    }
    let owner = if owner_override.is_empty() { None } else { Some(owner_override) };
    let repo = if repo_override.is_empty() { None } else { Some(repo_override) };
    let code = ksu::integrate(&app, &cfg, &provider, &git_ref, &method, owner, repo).await?;
    let _ = config::update(&app, |c| {
        c.ksu.provider = provider.clone();
        c.ksu.branch = git_ref.clone();
        c.ksu.method = method.clone();
    });
    Ok(format!("KernelSU 集成执行完成（setup.sh 退出码 {code}）"))
}

#[tauri::command]
pub fn ksu_status(state: State<'_, AppState>) -> KsuStatus {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return KsuStatus {
            message: "未选择内核目录".into(),
            ..Default::default()
        };
    }
    ksu::detect(Path::new(&cfg.general.kernel_dir))
}

#[tauri::command]
pub async fn ksu_manager_assets(
    state: State<'_, AppState>,
    repo: String,
) -> R<Vec<ReleaseAsset>> {
    let cfg = state.config_snapshot();
    ksu::list_manager_assets(&repo, &cfg.github.token, &cfg.github.api_base).await
}

#[tauri::command]
pub async fn ksu_download_manager(
    app: AppHandle,
    state: State<'_, AppState>,
    asset: ReleaseAsset,
) -> R<String> {
    let cfg = state.config_snapshot();
    let path = ksu::download_manager(&app, &cfg, &asset).await?;
    let _ = config::update(&app, |c| c.ksu.manager_path = path.clone());
    Ok(path)
}

/* ============================ 构建 ============================ */

#[tauri::command]
pub fn build_preview(state: State<'_, AppState>) -> String {
    let cfg = state.config_snapshot();
    let version = if cfg.general.kernel_dir.is_empty() {
        String::new()
    } else {
        kernel::detect(Path::new(&cfg.general.kernel_dir)).version
    };
    let plan = toolchain::plan(&cfg, &version);
    crate::build::generate_script(Path::new(&cfg.general.kernel_dir), &plan, &cfg.build)
}

#[tauri::command]
pub async fn build_start(app: AppHandle, state: State<'_, AppState>) -> R<i32> {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return Err("未选择内核目录".into());
    }
    let kernel_dir = Path::new(&cfg.general.kernel_dir);
    let workspace = Path::new(&cfg.general.workspace_dir);
    let version = kernel::detect(kernel_dir).version;
    let plan = toolchain::plan(&cfg, &version);
    let (code, _) = crate::build::start(&app, kernel_dir, workspace, &plan, &cfg.build).await?;
    Ok(code)
}

#[tauri::command]
pub fn build_clean(app: AppHandle, state: State<'_, AppState>) -> R<()> {
    let cfg = state.config_snapshot();
    crate::build::clean(&app, Path::new(&cfg.general.kernel_dir), &cfg.build)
}

#[tauri::command]
pub fn build_output_path(state: State<'_, AppState>) -> String {
    let cfg = state.config_snapshot();
    crate::build::output_dir(Path::new(&cfg.general.kernel_dir), &cfg.build)
        .to_string_lossy()
        .to_string()
}

/// 扫描产物目录，返回 [名称, 路径, 大小] 三元组（camelCase 由前端映射）
#[tauri::command]
pub fn build_artifacts(state: State<'_, AppState>) -> Vec<ArtifactInfo> {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return Vec::new();
    }
    crate::build::find_artifacts(Path::new(&cfg.general.kernel_dir), &cfg.build)
        .into_iter()
        .map(|(name, path, size)| ArtifactInfo { name, path, size })
        .collect()
}

/// 打包 AnyKernel3（构建成功后单独调用）
#[tauri::command]
pub async fn build_package(app: AppHandle, state: State<'_, AppState>) -> R<String> {
    let cfg = state.config_snapshot();
    if cfg.general.kernel_dir.is_empty() {
        return Err("未选择内核目录".into());
    }
    let kernel_dir = Path::new(&cfg.general.kernel_dir);
    let workspace = Path::new(&cfg.general.workspace_dir);
    std::fs::create_dir_all(workspace).ok();
    let p = crate::build::package_anykernel(&app, kernel_dir, workspace, &cfg.build).await?;
    let path = p.to_string_lossy().to_string();
    log_success(&app, "build", &format!("刷机包已生成：{path}"));
    Ok(path)
}

/* ============================ GitHub Actions ============================ */

#[tauri::command]
pub async fn gh_list_workflows(state: State<'_, AppState>) -> R<Vec<GhWorkflow>> {
    let cfg = state.config_snapshot();
    github::list_workflows(
        &cfg.github.owner,
        &cfg.github.repo,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await
}

#[tauri::command]
pub async fn gh_dispatch(
    app: AppHandle,
    state: State<'_, AppState>,
    workflow_id: String,
    git_ref: String,
    inputs: JsonValue,
) -> R<String> {
    let cfg = state.config_snapshot();
    let msg = github::dispatch_workflow(
        &cfg.github.owner,
        &cfg.github.repo,
        &workflow_id,
        &git_ref,
        &inputs,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await?;
    log_success(&app, "actions", &msg);
    Ok(msg)
}

#[tauri::command]
pub async fn gh_list_runs(state: State<'_, AppState>) -> R<Vec<GhRun>> {
    let cfg = state.config_snapshot();
    github::list_runs(
        &cfg.github.owner,
        &cfg.github.repo,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await
}

#[tauri::command]
pub async fn gh_list_artifacts(state: State<'_, AppState>, run_id: u64) -> R<Vec<GhArtifact>> {
    let cfg = state.config_snapshot();
    github::list_artifacts(
        &cfg.github.owner,
        &cfg.github.repo,
        run_id,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await
}

#[tauri::command]
pub async fn gh_download_artifact(
    app: AppHandle,
    state: State<'_, AppState>,
    artifact: GhArtifact,
) -> R<String> {
    let cfg = state.config_snapshot();
    let dest = Path::new(&cfg.general.workspace_dir).join("artifacts");
    github::download_artifact(
        &app,
        &cfg.github.owner,
        &cfg.github.repo,
        &artifact,
        &dest,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await
}

/* ============================ 杂项 ============================ */

#[tauri::command]
pub fn set_kernel_dir(app: AppHandle, path: String) -> R<KernelInfo> {
    let info = kernel::detect(Path::new(&path));
    let _ = config::update(&app, |c| c.general.kernel_dir = path.clone());
    Ok(info)
}

#[tauri::command]
pub async fn gh_cancel_run(
    state: State<'_, AppState>,
    run_id: u64,
) -> R<String> {
    let cfg = state.config_snapshot();
    github::cancel_run(
        &cfg.github.owner,
        &cfg.github.repo,
        run_id,
        &cfg.github.token,
        &cfg.github.api_base,
    )
    .await
}

#[tauri::command]
pub async fn test_mirror(state: State<'_, AppState>, id: String) -> R<CommandResult> {
    let cfg = state.config_snapshot();
    let mirror = cfg
        .mirror
        .mirrors
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("未知镜像：{id}"))?;

    let test_url = mirror::rewrite("https://api.github.com", mirror);
    let client = crate::net::client()?;
    match client.head(&test_url).send().await {
        Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 404 => {
            Ok(CommandResult {
                ok: true,
                code: 0,
                message: format!("镜像 {id} 连通正常（状态 {}）", resp.status()),
                output: String::new(),
            })
        }
        Ok(resp) => Ok(CommandResult {
            ok: false,
            code: resp.status().as_u16() as i32,
            message: format!("镜像 {id} 返回异常状态：{}", resp.status()),
            output: String::new(),
        }),
        Err(e) => Ok(CommandResult {
            ok: false,
            code: -1,
            message: format!("镜像 {id} 请求失败：{e}"),
            output: String::new(),
        }),
    }
}

#[tauri::command]
pub fn system_info() -> JsonValue {
    serde_json::json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpus": crate::proc::cpu_count(),
        "home": std::env::var("HOME").unwrap_or_default(),
    })
}
