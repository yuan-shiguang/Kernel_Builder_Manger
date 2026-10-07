//! Kernel Builder Manager —— Tauri 2.0 后端入口

mod bootstrap;
mod build;
mod commands;
mod config;
mod git;
mod github;
mod kernel;
mod ksu;
mod log;
mod mirror;
mod model;
mod net;
mod proc;
mod state;
mod susfs;
mod toolchain;

use std::time::Duration;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::new())
        .setup(|app| {
            let handle = app.handle().clone();

            // 读取（或创建）配置
            let cfg = config::load(&handle);

            // 首次初始化放到后台线程，前端有足够时间订阅日志事件
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(1200));
                bootstrap::ensure(&handle, &cfg);
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            /* 配置 / 系统 */
            commands::get_config,
            commands::save_config,
            commands::reset_config,
            commands::bootstrap_now,
            commands::host_dependencies,
            commands::open_path,
            commands::cancel,
            commands::system_info,
            /* 源码 */
            commands::detect_ssh,
            commands::preview_clone_url,
            commands::clone_kernel,
            commands::scan_kernel,
            commands::set_kernel_dir,
            commands::list_defconfigs,
            commands::read_defconfig,
            commands::write_defconfig,
            /* 工具链 */
            commands::toolchain_list,
            commands::toolchain_plan,
            commands::toolchain_install,
            commands::toolchain_install_missing,
            /* SUSFS */
            commands::susfs_fetch,
            commands::susfs_list,
            commands::susfs_apply,
            /* KSU */
            commands::ksu_providers,
            commands::ksu_branches,
            commands::ksu_fetch,
            commands::ksu_integrate,
            commands::ksu_status,
            commands::ksu_manager_assets,
            commands::ksu_download_manager,
            /* 构建 */
            commands::build_preview,
            commands::build_start,
            commands::build_clean,
            commands::build_output_path,
            commands::build_artifacts,
            commands::build_package,
            /* GitHub Actions */
            commands::gh_list_workflows,
            commands::gh_dispatch,
            commands::gh_list_runs,
            commands::gh_list_artifacts,
            commands::gh_download_artifact,
            commands::gh_cancel_run,
            commands::test_mirror,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Tauri 应用失败");
}
