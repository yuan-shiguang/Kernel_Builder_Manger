//! Git 相关：SSH Key 探测、协议自动选择、浅/完整克隆、分支切换

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use tauri::AppHandle;

use crate::log::{log_info, log_success, log_warn};
use crate::proc::{run_capture, run_streamed};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshStatus {
    pub has_key: bool,
    pub keys: Vec<String>,
    pub agent_loaded: bool,
    pub github_host_configured: bool,
    pub recommend: String,
}

/// 探测本机是否配置了可用于 GitHub 的 SSH Key
pub fn detect_ssh() -> SshStatus {
    let home = std::env::var("HOME").unwrap_or_default();
    let ssh_dir = Path::new(&home).join(".ssh");

    let mut keys: Vec<String> = Vec::new();
    for name in ["id_ed25519", "id_rsa", "id_ecdsa", "id_ed25519_sk", "id_ecdsa_sk"] {
        let priv_key = ssh_dir.join(name);
        let pub_key = ssh_dir.join(format!("{name}.pub"));
        if priv_key.exists() || pub_key.exists() {
            keys.push(name.to_string());
        }
    }

    let agent_loaded = run_capture("ssh-add", &["-l"])
        .map(|(code, out, _)| code == 0 && out.to_lowercase().contains("sha256"))
        .unwrap_or(false);

    let github_host_configured = std::fs::read_to_string(ssh_dir.join("config"))
        .map(|c| c.to_lowercase().contains("github.com"))
        .unwrap_or(false);

    let has_key = !keys.is_empty() || agent_loaded;

    SshStatus {
        has_key,
        keys,
        agent_loaded,
        github_host_configured,
        recommend: if has_key { "ssh" } else { "https" }.to_string(),
    }
}

/// 解析 owner/repo
pub fn parse_repo(input: &str) -> Option<(String, String)> {
    crate::mirror::normalize_repo(input)
}

/// 构造 clone 地址；protocol 为 auto 时按 SSH Key 检测结果决定
pub fn build_clone_url(input: &str, protocol: &str, ssh: &SshStatus) -> Result<String, String> {
    let (owner, repo) = parse_repo(input)
        .ok_or_else(|| format!("无法解析仓库地址：{input}（应为 owner/repo 或完整 URL）"))?;
    let repo_path = format!("{owner}/{repo}");
    let p = match protocol {
        "https" => "https",
        "ssh" => "ssh",
        _ => ssh.recommend.as_str(),
    };
    Ok(match p {
        "ssh" => crate::mirror::clone_ssh(&repo_path),
        _ => crate::mirror::clone_https(&repo_path),
    })
}

/// 内核源码克隆（浅克隆开关由调用方控制）
pub fn clone(
    app: &AppHandle,
    repo_input: &str,
    branch: &str,
    protocol: &str,
    shallow: bool,
    depth: u32,
    single_branch: bool,
    submodules: bool,
    dest: &Path,
) -> Result<(i32, String), String> {
    let ssh = detect_ssh();
    let url = build_clone_url(repo_input, protocol, &ssh)?;

    log_info(
        app,
        "clone",
        &format!(
            "SSH Key：{}（{} 个密钥，agent={}）",
            if ssh.has_key { "已配置" } else { "未配置" },
            ssh.keys.len(),
            ssh.agent_loaded
        ),
    );
    log_info(
        app,
        "clone",
        &format!(
            "协议：{} → {url}",
            if url.starts_with("git@") { "SSH（检测到本地 Key）" } else { "HTTPS" }
        ),
    );

    if is_repo(dest) {
        log_warn(app, "clone", "目录已是 git 仓库，改为增量更新");
        return fetch_all(app, dest);
    }
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("目标目录已存在且非空：{}", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut args: Vec<String> = vec!["clone".into(), "-v".into(), "--progress".into()];
    if shallow {
        args.push(format!("--depth={}", if depth == 0 { 1 } else { depth }));
        log_info(app, "clone", "浅克隆：--depth 已开启（可在设置中关闭）");
    } else {
        log_info(app, "clone", "完整克隆：不使用 --depth");
    }
    if !branch.is_empty() {
        args.push("--branch".into());
        args.push(branch.into());
        if single_branch {
            args.push("--single-branch".into());
        }
    }
    args.push(url);
    args.push(dest.to_string_lossy().to_string());

    let envs = HashMap::new();
    let (code, out) = run_streamed(app, "clone", "git", &args, None, &envs)?;
    if code != 0 {
        return Ok((code, out));
    }

    if submodules {
        log_info(app, "clone", "拉取子模块 …");
        let mut sub = vec![
            "submodule".into(),
            "update".into(),
            "--init".into(),
            "--recursive".into(),
        ];
        if shallow {
            sub.push("--depth=1".into());
        }
        run_streamed(app, "clone", "git", &sub, Some(dest), &envs)?;
    }

    log_success(app, "clone", &format!("源码就绪：{}", dest.display()));
    Ok((0, out))
}

/// 完整克隆（不带 --depth）
pub fn clone_full(app: &AppHandle, url: &str, dest: &Path) -> Result<(i32, String), String> {
    if is_repo(dest) {
        log_info(app, "clone", "仓库已存在，执行增量更新");
        return fetch_all(app, dest);
    }
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(false) {
        return Err(format!("目标目录已存在且非空：{}", dest.display()));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let args = vec![
        "clone".into(),
        "-v".into(),
        "--progress".into(),
        url.into(),
        dest.to_string_lossy().to_string(),
    ];
    log_info(app, "clone", &format!("完整克隆（不浅克隆）：{url}"));
    run_streamed(app, "clone", "git", &args, None, &HashMap::new())
}

/// 拉取全部分支与标签
pub fn fetch_all(app: &AppHandle, dir: &Path) -> Result<(i32, String), String> {
    let args = vec![
        "fetch".into(),
        "--all".into(),
        "--tags".into(),
        "--prune".into(),
        "--progress".into(),
    ];
    run_streamed(app, "clone", "git", &args, Some(dir), &HashMap::new())
}

/// 切换分支 / 标签（本地不存在时自动建立远端追踪）
pub fn checkout(app: &AppHandle, dir: &Path, git_ref: &str) -> Result<(i32, String), String> {
    let envs = HashMap::new();
    let (code, _) = run_streamed(
        app,
        "clone",
        "git",
        &["checkout".into(), git_ref.into()],
        Some(dir),
        &envs,
    )?;
    if code == 0 {
        return Ok((code, String::new()));
    }
    let _ = run_streamed(
        app,
        "clone",
        "git",
        &["fetch".into(), "origin".into(), git_ref.into(), "--progress".into()],
        Some(dir),
        &envs,
    )?;
    run_streamed(
        app,
        "clone",
        "git",
        &[
            "checkout".into(),
            "-B".into(),
            git_ref.into(),
            format!("origin/{git_ref}"),
        ],
        Some(dir),
        &envs,
    )
}

/// 列出本地仓库的全部远端分支（完整克隆后无需调用 API）
pub fn list_remote_branches(dir: &Path) -> Vec<String> {
    let Some((_, out, _)) = run_capture("git", &["branch", "-r", "--format=%(refname:short)"])
        .map(|v| v)
    else {
        return Vec::new();
    };
    out.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.contains("HEAD"))
        .filter_map(|l| l.strip_prefix("origin/").map(|s| s.to_string()))
        .collect()
}

pub fn current_branch(dir: &Path) -> String {
    run_capture("git", &["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|(_, out, _)| out.trim().to_string())
        .unwrap_or_default()
}

pub fn is_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}
