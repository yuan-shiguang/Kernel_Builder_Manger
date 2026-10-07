//! 配置持久化：<app_data_dir>/config.json

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::model::AppConfig;
use crate::state::AppState;

pub fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录：{e}"))
}

pub fn config_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("config.json"))
}

/// 启动时载入配置；不存在则写入默认配置
pub fn load(app: &AppHandle) -> AppConfig {
    let path = match config_file(app) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[config] {e}");
            return AppConfig::default();
        }
    };

    let cfg = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<AppConfig>(&text).unwrap_or_else(|e| {
                eprintln!("[config] 解析失败，回退默认配置：{e}");
                AppConfig::default()
            }),
            Err(e) => {
                eprintln!("[config] 读取失败：{e}");
                AppConfig::default()
            }
        }
    } else {
        let cfg = AppConfig::default();
        let _ = save(app, &cfg);
        cfg
    };

    if let Ok(p) = config_file(app) {
        *app.state::<AppState>().config_path.lock().unwrap() = Some(p);
    }
    *app.state::<AppState>().config.lock().unwrap() = cfg.clone();
    cfg
}

pub fn save(app: &AppHandle, cfg: &AppConfig) -> Result<(), String> {
    let path = config_file(app)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建配置目录失败：{e}"))?;
    }
    let text = serde_json::to_string_pretty(cfg).map_err(|e| format!("序列化配置失败：{e}"))?;
    std::fs::write(&path, text).map_err(|e| format!("写入配置失败：{e}"))?;
    *app.state::<AppState>().config.lock().unwrap() = cfg.clone();
    Ok(())
}

/// 更新并落盘
pub fn update<F: FnOnce(&mut AppConfig)>(app: &AppHandle, f: F) -> Result<AppConfig, String> {
    let snapshot = app.state::<AppState>().update_config(f);
    save(app, &snapshot)?;
    Ok(snapshot)
}
