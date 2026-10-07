//! SUSFS 补丁：获取 NonGKI_Kernel_Build_2nd、枚举补丁、应用并输出失败详情

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::log::{log_error, log_info, log_stream, log_success, log_warn};
use crate::mirror;
use crate::model::{AppConfig, PatchApplyResult, PatchFileInfo, RejectInfo};
use crate::net::{collect_files, download_to_file, extract_tar_gz, head_lines};
use crate::proc::run_capture;

/// SUSFS 本地目录：完整克隆时按仓库命名（分支用 checkout 切换）
pub fn susfs_dir(cfg: &AppConfig) -> PathBuf {
    let key = mirror::normalize_repo(&cfg.susfs.repo)
        .map(|(o, r)| format!("{o}_{r}"))
        .unwrap_or_else(|| cfg.susfs.repo.replace('/', "_"));
    PathBuf::from(&cfg.general.workspace_dir).join("susfs").join(key)
}

/// 获取补丁仓库。
/// full_clone = true 时做完整 git 克隆（不浅克隆），可随时切换分支；
/// 否则只下载当前分支的 tarball。
pub async fn fetch(app: &AppHandle, cfg: &AppConfig) -> Result<String, String> {
    let (owner, repo) = mirror::normalize_repo(&cfg.susfs.repo)
        .ok_or_else(|| "SUSFS 仓库地址无效，应为 owner/repo".to_string())?;
    let repo_path = format!("{owner}/{repo}");
    let root = PathBuf::from(&cfg.general.workspace_dir).join("susfs");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    if cfg.susfs.full_clone {
        let dir = susfs_dir(cfg);
        let raw = mirror::clone_https(&repo_path);
        let url = match mirror::active_mirror(&cfg.mirror) {
            Some(m) => mirror::rewrite(&raw, m),
            None => raw,
        };
        if crate::git::is_repo(&dir) {
            log_info(app, "susfs", "仓库已存在，增量更新（完整历史）…");
            crate::git::fetch_all(app, &dir)?;
        } else {
            log_info(app, "susfs", &format!("完整克隆 SUSFS 仓库（不浅克隆）：{url}"));
            crate::git::clone_full(app, &url, &dir)?;
        }
        crate::git::checkout(app, &dir, &cfg.susfs.branch)?;
        log_success(app, "susfs", &format!("SUSFS 就绪：{}", dir.display()));
        return Ok(dir.to_string_lossy().to_string());
    }

    let dest = susfs_dir(cfg).join(&cfg.susfs.branch);
    let tmp = root.join(format!("{owner}_{repo}-{}.tar.gz", cfg.susfs.branch));
    let url = mirror::codeload_url(&repo_path, &cfg.susfs.branch);
    log_info(app, "susfs", &format!("下载 SUSFS 补丁包：{url}"));
    download_to_file(app, &cfg.mirror, &url, &tmp, "SUSFS 补丁仓库", "susfs").await?;
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    extract_tar_gz(&tmp, &dest)?;
    let _ = std::fs::remove_file(&tmp);
    log_success(app, "susfs", &format!("SUSFS 已下载到 {}", dest.display()));
    Ok(dest.to_string_lossy().to_string())
}

/// 枚举补丁文件
pub fn list_patches(cfg: &AppConfig) -> Vec<PatchFileInfo> {
    let root = susfs_dir(cfg);
    if !root.exists() {
        return Vec::new();
    }
    collect_files(&root, &[".patch", ".diff"])
        .into_iter()
        .map(|p| {
            let size = p.metadata().map(|m| m.len()).unwrap_or(0);
            let hint = head_lines(&p, 12)
                .lines()
                .find(|l| l.starts_with("+++") || l.starts_with("---"))
                .unwrap_or("")
                .trim()
                .to_string();
            PatchFileInfo {
                name: p
                    .strip_prefix(&root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string(),
                path: p.to_string_lossy().to_string(),
                size,
                hint,
            }
        })
        .collect()
}

/// 枚举可直接复制的 SUSFS 源码目录
pub fn list_sources(cfg: &AppConfig) -> Vec<String> {
    let root = susfs_dir(cfg);
    if !root.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if name.contains("susfs") {
                out.push(p.strip_prefix(&root).unwrap_or(&p).to_string_lossy().to_string());
            }
            if name != ".git" {
                stack.push(p);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// 应用单个补丁（SUSFS / KSU 共用）
pub fn apply_patch_file(
    app: &AppHandle,
    kernel_dir: &Path,
    patch: &Path,
    method: &str,
) -> PatchApplyResult {
    let mut res = PatchApplyResult::default();
    let name = patch.file_name().unwrap_or_default().to_string_lossy().to_string();
    let is_git = crate::git::is_repo(kernel_dir);

    let order: Vec<&str> = match method {
        "git" => vec!["git"],
        "patch" => vec!["patch"],
        _ => {
            if is_git {
                vec!["git", "patch"]
            } else {
                vec!["patch", "git"]
            }
        }
    };

    for m in order {
        log_info(app, "patch", &format!("尝试以 {m} 应用 {name}"));
        let (code, out, err) = if m == "git" {
            run_capture(
                "git",
                &["apply", "--3way", "--whitespace=nowarn", patch.to_string_lossy().as_ref()],
            )
            .unwrap_or((-1, String::new(), "无法执行 git".into()))
        } else {
            run_capture(
                "patch",
                &[
                    "-p1",
                    "-f",
                    "--no-backup-if-mismatch",
                    "-i",
                    patch.to_string_lossy().as_ref(),
                ],
            )
            .unwrap_or((-1, String::new(), "无法执行 patch".into()))
        };

        let combined = format!("{out}\n{err}");
        for line in combined.lines().filter(|l| !l.trim().is_empty()) {
            log_stream(app, "patch", "stdout", line);
        }
        res.log.push_str(&combined);
        res.method = m.to_string();
        res.exit_code = code;

        if code == 0 {
            res.ok = true;
            res.applied.push(name.clone());
            log_success(app, "patch", &format!("{name} 应用成功"));
            return res;
        }
        res.failed.push(name.clone());
    }

    res.ok = false;
    res.rejects = collect_rejects(kernel_dir);
    res.detail = build_failure_detail(kernel_dir, patch, &res);
    res.message = format!(
        "补丁 {name} 应用失败（方式 {}，退出码 {}）",
        res.method, res.exit_code
    );
    log_error(app, "patch", &res.message);
    res
}

/// 批量应用补丁：逐个执行，失败可按 stop_on_error 中断；
/// 结果（含失败详情）一律返回，便于前端渲染「失败详情」面板。
pub fn apply_many(
    app: &AppHandle,
    kernel_dir: &Path,
    patches: &[PathBuf],
    method: &str,
    keep_rejects: bool,
    stop_on_error: bool,
) -> Vec<PatchApplyResult> {
    let mut results = Vec::with_capacity(patches.len());
    for p in patches {
        let r = apply_patch_file(app, kernel_dir, p, method);
        let failed = !r.ok;
        results.push(r);
        if failed && stop_on_error {
            break;
        }
    }
    if !keep_rejects {
        let _ = clean_rejects(kernel_dir);
    }
    results
}

fn collect_rejects(kernel_dir: &Path) -> Vec<RejectInfo> {
    let mut out = Vec::new();
    let mut stack = vec![kernel_dir.to_path_buf()];
    let mut guard = 0usize;
    while let Some(d) = stack.pop() {
        if guard > 200 {
            break;
        }
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                let n = p
                    .file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !matches!(n.as_str(), ".git" | "out" | "node_modules") {
                    stack.push(p);
                }
            } else if p.extension().and_then(|x| x.to_str()) == Some("rej") {
                guard += 1;
                out.push(RejectInfo {
                    file: p.strip_prefix(kernel_dir).unwrap_or(&p).to_string_lossy().to_string(),
                    content: std::fs::read_to_string(&p)
                        .unwrap_or_default()
                        .chars()
                        .take(4000)
                        .collect(),
                });
            }
        }
    }
    out
}

fn build_failure_detail(kernel_dir: &Path, patch: &Path, res: &PatchApplyResult) -> String {
    let mut s = String::new();
    s.push_str(&format!("补丁文件：{}\n", patch.display()));
    s.push_str(&format!("内核目录：{}\n", kernel_dir.display()));
    s.push_str(&format!("使用方式：{}，退出码：{}\n\n", res.method, res.exit_code));

    if let Some((_, _, err)) = run_capture(
        "git",
        &["apply", "--check", "--verbose", patch.to_string_lossy().as_ref()],
    ) {
        if !err.trim().is_empty() {
            s.push_str("—— git apply --check ——\n");
            s.push_str(err.trim());
            s.push_str("\n\n");
        }
    }

    s.push_str("—— 原始输出 ——\n");
    s.push_str(res.log.trim());
    s.push_str("\n\n");

    if res.rejects.is_empty() {
        s.push_str("未生成 .rej 文件（补丁可能在检查阶段即被拒绝，通常是上下文不匹配或已应用过）。\n");
    } else {
        s.push_str(&format!("共生成 {} 个 .rej 冲突文件：\n", res.rejects.len()));
        for r in &res.rejects {
            s.push_str(&format!("\n### {}\n{}\n", r.file, r.content));
        }
    }

    s.push_str("\n—— 排查建议 ——\n");
    s.push_str("1. 确认补丁版本与内核 Linux 版本匹配（不同版本的 SUSFS 补丁不通用）；\n");
    s.push_str("2. 确认已先集成 KernelSU（SUSFS 依赖 KSU 的挂载点）；\n");
    s.push_str("3. 若补丁已部分应用，先在「补丁」页执行回滚再重试；\n");
    s.push_str("4. 内核若是浅克隆（--depth=1），git apply --3way 会因缺少历史而失败，请关闭浅克隆重新拉取。\n");
    s
}

/// 清理 .rej / .orig
pub fn clean_rejects(kernel_dir: &Path) -> usize {
    let Some((_, out, _)) = run_capture(
        "sh",
        &[
            "-c",
            &format!("find '{}' \\( -name '*.rej' -o -name '*.orig' \\) -print -delete", kernel_dir.display()),
        ],
    ) else {
        return 0;
    };
    out.lines().filter(|l| !l.trim().is_empty()).count()
}

/// 回滚内核源码（仅限 git 仓库）
pub fn revert(app: &AppHandle, kernel_dir: &Path) -> Result<i32, String> {
    if !crate::git::is_repo(kernel_dir) {
        log_warn(app, "patch", "非 git 仓库，无法自动回滚，请手动恢复");
        return Ok(-1);
    }
    let Some((code, _, err)) = run_capture("git", &["checkout", "--", "."]) else {
        return Err("执行 git 失败".into());
    };
    if code != 0 {
        log_error(app, "patch", &format!("回滚失败：{err}"));
    } else {
        log_success(app, "patch", "已回滚内核源码到 HEAD");
    }
    let _ = clean_rejects(kernel_dir);
    Ok(code)
}
