//! 首次初始化：创建工作区、检查主机依赖、下载必备工具链

use std::path::Path;

use tauri::AppHandle;

use crate::config;
use crate::log::{log_error, log_info, log_success, log_warn};
use crate::model::AppConfig;
use crate::toolchain;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapReport {
    pub workspace: String,
    pub created_dirs: Vec<String>,
    pub missing_deps: Vec<String>,
    pub install_hint: String,
    pub downloaded: Vec<String>,
    pub failed: Vec<String>,
    pub ok: bool,
}

/// 执行初始化。force=true 时即使已初始化也会重跑工具链检查
pub fn run(app: &AppHandle, cfg: &AppConfig, force: bool) -> BootstrapReport {
    let mut report = BootstrapReport {
        workspace: cfg.general.workspace_dir.clone(),
        created_dirs: vec![],
        missing_deps: vec![],
        install_hint: String::new(),
        downloaded: vec![],
        failed: vec![],
        ok: true,
    };

    let workspace = Path::new(&cfg.general.workspace_dir);
    let toolchain_dir = Path::new(&cfg.toolchain.install_dir);

    for dir in [workspace, toolchain_dir, &workspace.join("ksu"), &workspace.join("out")] {
        if !dir.exists() {
            match std::fs::create_dir_all(dir) {
                Ok(_) => report.created_dirs.push(dir.display().to_string()),
                Err(e) => {
                    log_error(app, "init", &format!("创建 {} 失败：{e}", dir.display()));
                    report.failed.push(format!("{}：{e}", dir.display()));
                }
            }
        }
    }
    log_info(app, "init", &format!("工作区：{}", workspace.display()));

    // ---- 主机依赖 ----
    let deps = toolchain::host_dependencies();
    let missing: Vec<String> = deps
        .iter()
        .filter(|d| d.required && !d.present)
        .map(|d| d.name.clone())
        .collect();
    report.missing_deps = missing.clone();
    report.install_hint = toolchain::install_command();
    if !missing.is_empty() {
        log_warn(
            app,
            "init",
            &format!("缺少主机依赖：{}\n建议执行：{}", missing.join(", "), report.install_hint),
        );
        report.ok = false;
    } else {
        log_success(app, "init", "主机依赖检查通过");
    }

    // ---- 必备工具链 ----
    if cfg.general.initialized && !force {
        log_info(app, "init", "已初始化过，跳过必备工具链下载");
        report.ok = report.ok && missing.is_empty();
        return report;
    }

    for item in toolchain::bootstrap_items(&cfg) {
        if item.installed {
            log_info(app, "init", &format!("{} 已存在", item.name));
            continue;
        }
        log_info(app, "init", &format!("初始化：准备 {}", item.name));
        match toolchain::install(app, &cfg, &item) {
            Ok(p) => {
                report.downloaded.push(format!("{} -> {}", item.name, p));
            }
            Err(e) => {
                log_error(app, "init", &e);
                report.failed.push(format!("{}：{e}", item.name));
            }
        }
    }

    if !report.failed.is_empty() {
        report.ok = false;
    }

    // 落盘 initialized
    let _ = config::update(app, |c| {
        c.general.initialized = report.ok;
        c.general.bootstrapped = report.downloaded.clone();
    });

    if report.ok {
        log_success(app, "init", "初始化完成");
    } else {
        log_error(app, "init", "初始化未完全成功，请查看上方日志");
    }

    report
}

/// 仅在未初始化时静默执行（启动时调用）
pub fn ensure(app: &AppHandle, cfg: &AppConfig) {
    if cfg.general.initialized {
        log_info(app, "init", "检测到已初始化，跳过首次引导");
        return;
    }
    log_info(app, "init", "首次启动，开始初始化 …");
    let report = run(app, cfg, false);
    log_info(
        app,
        "init",
        &format!(
            "初始化结果：ok={}，已下载 {} 项，失败 {} 项",
            report.ok,
            report.downloaded.len(),
            report.failed.len()
        ),
    );
}
