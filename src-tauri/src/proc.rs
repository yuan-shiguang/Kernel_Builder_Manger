//! 子进程执行器：实时把 stdout/stderr 推到前端日志面板，并支持取消

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use tauri::AppHandle;

use crate::log::{log_error, log_stream, log_success, log_warn};
use crate::state::AppState;

/// 流式运行命令，返回 (退出码, 合并输出)
pub fn run_streamed(
    app: &AppHandle,
    task: &str,
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    envs: &HashMap<String, String>,
) -> Result<(i32, String), String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .envs(envs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // 固定 C 语言环境，保证错误信息关键字可被识别
    cmd.env("LC_ALL", "C").env("LANG", "C");

    if let Some(dir) = cwd {
        if dir.exists() {
            cmd.current_dir(dir);
        } else {
            return Err(format!("工作目录不存在：{}", dir.display()));
        }
    }

    log_cmd(app, task, program, args);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("无法启动 {program}：{e}（请确认已安装并加入 PATH）"))?;

    app.state::<AppState>().register_task(task, child.id());

    let (tx, rx) = mpsc::channel::<String>();
    if let Some(out) = child.stdout.take() {
        spawn_reader(app, task, "stdout", out, tx.clone());
    }
    if let Some(err) = child.stderr.take() {
        spawn_reader(app, task, "stderr", err, tx.clone());
    }
    drop(tx);

    let collector = thread::spawn(move || {
        let mut buf = String::new();
        for line in rx {
            buf.push_str(&line);
            buf.push('\n');
        }
        buf
    });

    let status = child.wait().map_err(|e| format!("等待进程结束失败：{e}"))?;
    let combined = collector.join().unwrap_or_default();
    let code = status.code().unwrap_or(-1);

    app.state::<AppState>().clear_task(task);

    if code == 0 {
        log_success(app, task, &format!("{program} 执行完成（退出码 0）"));
    } else {
        log_error(app, task, &format!("{program} 执行失败，退出码 {code}"));
    }
    Ok((code, combined))
}

fn log_cmd(app: &AppHandle, task: &str, program: &str, args: &[String]) {
    let shown: Vec<String> = args.iter().take(14).cloned().collect();
    let suffix = if args.len() > 14 { " …" } else { "" };
    log_stream(
        app,
        task,
        "system",
        &format!("$ {program} {}{}", shown.join(" "), suffix),
    );
}

fn spawn_reader<R: Read + Send + 'static>(
    app: &AppHandle,
    task: &str,
    pipe: R,
    tx: mpsc::Sender<String>,
) {
    let app = app.clone();
    let task = task.to_string();
    thread::spawn(move || {
        let reader = BufReader::new(pipe);
        for line in reader.lines() {
            match line {
                Ok(text) => {
                    log_stream(&app, &task, "stdout", &text);
                    let _ = tx.send(text);
                }
                Err(_) => break,
            }
        }
    });
}

/// 取消任务：先 TERM 再 KILL（先尝试进程组）
pub fn cancel_task(app: &AppHandle, task: &str) -> bool {
    let pid = match app.state::<AppState>().task_pid(task) {
        Some(p) => p,
        None => {
            log_warn(app, task, "没有正在运行的任务可取消");
            return false;
        }
    };
    log_warn(app, task, &format!("正在终止进程 {pid} …"));
    let pid_s = pid.to_string();
    let pgid = format!("-{pid}");
    let _ = Command::new("kill").args(["-TERM", &pgid]).status();
    let _ = Command::new("kill").args(["-TERM", &pid_s]).status();
    thread::sleep(std::time::Duration::from_millis(800));
    let _ = Command::new("kill").args(["-KILL", &pgid]).status();
    let _ = Command::new("kill").args(["-KILL", &pid_s]).status();
    app.state::<AppState>().clear_task(task);
    log_warn(app, task, "任务已取消");
    true
}

/// 轻量执行并取回 (code, stdout, stderr)
pub fn run_capture(program: &str, args: &[&str]) -> Option<(i32, String, String)> {
    let out = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    Some((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    ))
}

pub fn which(name: &str) -> bool {
    run_capture("sh", &["-c", &format!("command -v {name} >/dev/null 2>&1")])
        .map(|(code, _, _)| code == 0)
        .unwrap_or(false)
}

/// CPU 逻辑核心数（用于 -j）
pub fn cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
