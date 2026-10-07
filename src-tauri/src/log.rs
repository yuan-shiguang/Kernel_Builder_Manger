//! 统一日志通道：后端所有输出都通过事件推给前端日志面板

use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::model::LogLine;
use crate::state::AppState;

pub const LOG_EVENT: &str = "log://line";
pub const PROGRESS_EVENT: &str = "progress://download";

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

const ERROR_HINTS: &[&str] = &[
    "error:",
    "error :",
    "error]",
    "fatal error",
    "fatal:",
    "***",
    "undefined reference",
    "command not found",
    "no such file or directory",
    "permission denied",
    "cannot find",
    "unrecognized command",
    "ld.lld: error",
    "clang: error",
    "gcc: error",
    "aborting",
    "failed to",
    "patch does not apply",
    "hunk failed",
    "conflict",
    "not a git repository",
    "could not read",
    "fatal: ",
];

const WARN_HINTS: &[&str] = &["warning:", "warning :", "warn:", "deprecated", "is deprecated"];

/// 根据文本特征判定日志等级，用于前端高亮
pub fn classify(line: &str) -> &'static str {
    let lower = line.to_lowercase();
    if ERROR_HINTS.iter().any(|h| lower.contains(h)) {
        return "error";
    }
    if WARN_HINTS.iter().any(|h| lower.contains(h)) {
        return "warn";
    }
    "info"
}

pub fn emit_log(app: &AppHandle, task: &str, stream: &str, level: &str, text: &str) {
    let seq = app.state::<AppState>().next_seq();
    let line = LogLine {
        seq,
        task: task.to_string(),
        stream: stream.to_string(),
        level: level.to_string(),
        text: text.to_string(),
        ts: now_ms(),
    };
    let _ = app.emit(LOG_EVENT, line);
}

pub fn log_info(app: &AppHandle, task: &str, text: impl AsRef<str>) {
    emit_log(app, task, "system", "info", text.as_ref());
}

pub fn log_warn(app: &AppHandle, task: &str, text: impl AsRef<str>) {
    emit_log(app, task, "system", "warn", text.as_ref());
}

pub fn log_error(app: &AppHandle, task: &str, text: impl AsRef<str>) {
    emit_log(app, task, "system", "error", text.as_ref());
}

pub fn log_success(app: &AppHandle, task: &str, text: impl AsRef<str>) {
    emit_log(app, task, "system", "success", text.as_ref());
}

/// 子进程输出：自动判定等级
pub fn log_stream(app: &AppHandle, task: &str, stream: &str, text: &str) {
    let level = classify(text);
    emit_log(app, task, stream, level, text);
}
