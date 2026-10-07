//! KernelSU 集成：覆盖官方与全部社区分支
//!
//! 分支定位（社区整理）：
//!   KernelSU      官方，metamodule 管理模块挂载；GKI 2.0（5.10+），v1.0 后移除非 GKI
//!   KernelSU-Next 增强版，Magic Mount + OverlayFS 切换，模块备份/自动更新；4.4–6.6
//!   SukiSU-Ultra  内置 SUSFS，支持 KPM；原名 MKSU-SKN，从 MKSU 分叉
//!   BakaSU        原 ReSukiSU，兼容多种管理器，从 SukiSU-Ultra 分叉
//!   ReSukiSU      cctv18 维护，fork 自 SukiSU/BakaSU
//!   rsuntk / rsuntk-SUSFS / xxKSU / Next 一并纳入
//!
//! 源码获取：**完整 git 克隆，不做浅克隆** —— 保留完整历史与 git 元数据，
//! 本地即拥有全部分支，切换分支无需再次联网，也不受 GitHub API 限流影响。

use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::git;
use crate::log::{log_error, log_info, log_stream, log_success, log_warn};
use crate::mirror;
use crate::model::{AppConfig, GitRefInfo, KsuProvider, KsuStatus, ReleaseAsset};
use crate::net::{download_to_file, get_json};

pub const PROVIDERS: &[KsuProvider] = &[
    KsuProvider {
        id: "kernelsu".into(),
        name: "KernelSU（官方）".into(),
        owner: "tiann".into(),
        repo: "KernelSU".into(),
        author: "tiann".into(),
        default_branch: "main".into(),
        description: "原始项目，metamodule 系统管理模块挂载；GKI 2.0（5.10+），v1.0 后不再支持非 GKI".into(),
        manager_repo: "tiann/KernelSU".into(),
        homepage: "https://github.com/tiann/KernelSU".into(),
    },
    KsuProvider {
        id: "next".into(),
        name: "KernelSU-Next".into(),
        owner: "rifsxd".into(),
        repo: "KernelSU-Next".into(),
        author: "rifsxd".into(),
        default_branch: "next".into(),
        description: "增强版分支，Magic Mount + OverlayFS 切换，模块备份与自动更新；支持 4.4–6.6（含非 GKI）".into(),
        manager_repo: "rifsxd/KernelSU-Next".into(),
        homepage: "https://github.com/rifsxd/KernelSU-Next".into(),
    },
    KsuProvider {
        id: "sukisu-ultra".into(),
        name: "SukiSU-Ultra".into(),
        owner: "ShirkNeko".into(),
        repo: "SukiSU-Ultra".into(),
        author: "ShirkNeko".into(),
        default_branch: "main".into(),
        description: "内核级能力，内置 SUSFS，支持 KPM（内核补丁模块）；原名 MKSU-SKN，从 MKSU 分叉".into(),
        manager_repo: "ShirkNeko/SukiSU-Ultra".into(),
        homepage: "https://github.com/ShirkNeko/SukiSU-Ultra".into(),
    },
    KsuProvider {
        id: "bakasu".into(),
        name: "BakaSU（原 ReSukiSU）".into(),
        owner: "Baka-SU".into(),
        repo: "BakaSU".into(),
        author: "BakaSU Development".into(),
        default_branch: "main".into(),
        description: "多管理器支持（KernelSU / MKSU / RKSU / SukiSU），metamodule 系统；从 SukiSU-Ultra 分叉，原名 ReSukiSU".into(),
        manager_repo: "Baka-SU/BakaSU".into(),
        homepage: "https://github.com/Baka-SU/BakaSU".into(),
    },
    KsuProvider {
        id: "rsuntk".into(),
        name: "rsuntk KernelSU".into(),
        owner: "rsuntk".into(),
        repo: "KernelSU".into(),
        author: "rsuntk".into(),
        default_branch: "main".into(),
        description: "rsuntk 维护的 KernelSU 分支（Personal fork of KernelSU Project）".into(),
        manager_repo: "rsuntk/KernelSU".into(),
        homepage: "https://github.com/rsuntk/KernelSU".into(),
    },
    KsuProvider {
        id: "rsuntk-susfs".into(),
        name: "rsuntk-SUSFS".into(),
        owner: "cyberc3dr".into(),
        repo: "KernelSU".into(),
        author: "cyberc3dr".into(),
        default_branch: "main".into(),
        description: "rsuntk 分支的 SUSFS 整合版本".into(),
        manager_repo: "cyberc3dr/KernelSU".into(),
        homepage: "https://github.com/cyberc3dr/KernelSU".into(),
    },
    KsuProvider {
        id: "xxksu".into(),
        name: "xxKSU".into(),
        owner: "backslashxx".into(),
        repo: "KernelSU".into(),
        author: "backslashxx".into(),
        default_branch: "main".into(),
        description: "backslashxx 的 KSU 分支，upstream 兼容驱动（Linux 3.0 – 5.4+）".into(),
        manager_repo: "backslashxx/KernelSU".into(),
        homepage: "https://github.com/backslashxx/KernelSU".into(),
    },
];

pub fn providers() -> Vec<KsuProvider> {
    PROVIDERS.to_vec()
}

pub fn find_provider(id: &str) -> Option<&'static KsuProvider> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// 本地缓存目录（一个仓库一份完整克隆）
pub fn cache_dir(cfg: &AppConfig, owner: &str, repo: &str) -> PathBuf {
    PathBuf::from(&cfg.general.workspace_dir)
        .join("ksu")
        .join(format!("{owner}_{repo}"))
}

/* --------------------------- 分支 / 标签 --------------------------- */

pub fn local_branches(cfg: &AppConfig, owner: &str, repo: &str) -> Vec<GitRefInfo> {
    let dir = cache_dir(cfg, owner, repo);
    if !git::is_repo(&dir) {
        return Vec::new();
    }
    git::list_remote_branches(&dir)
        .into_iter()
        .map(|name| GitRefInfo {
            name,
            sha: String::new(),
            kind: "branch".into(),
        })
        .collect()
}

pub async fn list_branches(
    owner: &str,
    repo: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<GitRefInfo>, String> {
    let url = format!("{api_base}/repos/{owner}/{repo}/branches?per_page=100");
    let v = get_json(&url, token).await?;
    Ok(v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|i| {
                    i.get("name").and_then(|n| n.as_str()).map(|name| GitRefInfo {
                        name: name.to_string(),
                        sha: i
                            .get("commit")
                            .and_then(|c| c.get("sha"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(8)
                            .collect(),
                        kind: "branch".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

pub async fn list_tags(
    owner: &str,
    repo: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<GitRefInfo>, String> {
    let url = format!("{api_base}/repos/{owner}/{repo}/tags?per_page=100");
    let v = get_json(&url, token).await?;
    Ok(v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|i| {
                    i.get("name").and_then(|n| n.as_str()).map(|name| GitRefInfo {
                        name: name.to_string(),
                        sha: i
                            .get("commit")
                            .and_then(|c| c.get("sha"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .chars()
                            .take(8)
                            .collect(),
                        kind: "tag".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/* --------------------------- 集成 --------------------------- */

/// 完整克隆（不浅克隆）并切换到目标分支
async fn fetch_source(
    app: &AppHandle,
    cfg: &AppConfig,
    owner: &str,
    repo: &str,
    label: &str,
    git_ref: &str,
) -> Result<PathBuf, String> {
    let dir = cache_dir(cfg, owner, repo);
    std::fs::create_dir_all(dir.parent().unwrap_or(Path::new("."))).ok();

    let raw = mirror::clone_https(&format!("{owner}/{repo}"));
    let url = match mirror::active_mirror(&cfg.mirror) {
        Some(m) => mirror::rewrite(&raw, m),
        None => raw,
    };

    if git::is_repo(&dir) {
        log_info(app, "ksu", &format!("{label} 已完整克隆，增量更新分支…"));
        git::fetch_all(app, &dir)?;
    } else {
        log_info(
            app,
            "ksu",
            &format!("完整克隆 {label}（{url}）—— 不使用 --depth，保留全部分支与历史"),
        );
        git::clone_full(app, &url, &dir)?;
    }

    log_info(app, "ksu", &format!("切换到分支 {git_ref}"));
    git::checkout(app, &dir, git_ref)?;
    Ok(dir)
}

pub async fn integrate(
    app: &AppHandle,
    cfg: &AppConfig,
    provider_id: &str,
    git_ref: &str,
    method: &str,
    owner_override: Option<String>,
    repo_override: Option<String>,
) -> Result<i32, String> {
    let preset = find_provider(provider_id);
    let owner = owner_override
        .filter(|s| !s.is_empty())
        .or_else(|| preset.map(|p| p.owner.clone()))
        .ok_or_else(|| "未知的 KernelSU 来源".to_string())?;
    let repo = repo_override
        .filter(|s| !s.is_empty())
        .or_else(|| preset.map(|p| p.repo.clone()))
        .ok_or_else(|| "未知的 KernelSU 来源".to_string())?;
    let label = preset
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("{owner}/{repo}"));

    let kernel_dir = PathBuf::from(&cfg.general.kernel_dir);
    if !kernel_dir.join("Makefile").exists() {
        return Err("请先选择有效的内核源码目录".to_string());
    }

    log_info(
        app,
        "ksu",
        &format!("集成 {label}（{owner}/{repo}）@ {git_ref}，方式：{method}"),
    );

    let src = fetch_source(app, cfg, &owner, &repo, &label, git_ref).await?;
    log_info(app, "ksu", &format!("源码就绪：{}", src.display()));

    match method {
        "manual" => integrate_manual(app, &src, &kernel_dir),
        _ => integrate_setup(app, &src, &kernel_dir, git_ref),
    }
}

/// 官方方式：执行 kernel/setup.sh <ref>
fn integrate_setup(
    app: &AppHandle,
    src: &Path,
    kernel_dir: &Path,
    git_ref: &str,
) -> Result<i32, String> {
    let setup = src.join("kernel").join("setup.sh");
    if !setup.exists() {
        log_warn(app, "ksu", "未找到 kernel/setup.sh，回退到手动复制方式");
        return integrate_manual(app, src, kernel_dir);
    }

    let args = vec![setup.to_string_lossy().to_string(), git_ref.to_string()];
    let envs = std::collections::HashMap::new();
    let (code, _) = crate::proc::run_streamed(
        app,
        "ksu",
        &setup.to_string_lossy(),
        &args,
        Some(kernel_dir),
        &envs,
    )
    .map_err(|e| e.to_string())?;

    if code != 0 {
        log_error(
            app,
            "ksu",
            &format!("setup.sh 执行失败（退出码 {code}），请查看日志中的错误行"),
        );
    } else {
        log_success(app, "ksu", "KernelSU 集成完成，请在「defconfig」页确认 CONFIG_KSU 已开启");
    }
    Ok(code)
}

/// 手动方式：复制 kernel/ 目录树 + 应用补丁 + 追加开关
fn integrate_manual(app: &AppHandle, src: &Path, kernel_dir: &Path) -> Result<i32, String> {
    let kdir = src.join("kernel");
    if !kdir.exists() {
        return Err("KSU 源码中未找到 kernel/ 目录，请确认分支选择正确".to_string());
    }

    let Some((code, _, err)) = crate::proc::run_capture(
        "sh",
        &[
            "-c",
            &format!("cp -rf '{}/.' '{}' 2>&1 | tail -20", kdir.display(), kernel_dir.display()),
        ],
    ) else {
        return Err("执行 cp 失败".into());
    };
    if code != 0 {
        log_error(app, "ksu", &format!("复制 KSU 源码失败：{err}"));
        return Ok(code);
    }
    log_info(app, "ksu", "已复制 kernel/ 目录树到内核源码");

    let patches = crate::net::collect_files(&kdir, &[".patch", ".diff"]);
    if patches.is_empty() {
        log_info(app, "ksu", "未发现附加补丁（纯复制式分支，属正常情况）");
    }
    for p in patches {
        log_info(app, "ksu", &format!("应用补丁 {}", p.display()));
        let r = crate::susfs::apply_patch_file(app, kernel_dir, &p, "auto");
        if !r.ok {
            log_error(app, "ksu", &format!("补丁 {} 失败：{}", p.display(), r.message));
        }
    }

    log_success(app, "ksu", "手动集成完成，请在「defconfig」页确认 CONFIG_KSU=y");
    Ok(0)
}

/* --------------------------- 状态检测 --------------------------- */

pub fn detect(kernel_dir: &Path) -> KsuStatus {
    let mut st = KsuStatus::default();
    if kernel_dir.as_os_str().is_empty() || !kernel_dir.exists() {
        st.message = "尚未选择内核源码目录".into();
        return st;
    }

    for c in [
        "drivers/kernelsu",
        "fs/kernelsu",
        "kernel/kernelsu",
        "drivers/kernel-su",
    ] {
        if kernel_dir.join(c).exists() {
            st.integrated = true;
            st.files.push(c.to_string());
        }
    }

    let mut search = vec![
        kernel_dir.join(".config"),
        kernel_dir.join("out").join(".config"),
    ];
    if let Ok(rd) = std::fs::read_dir(kernel_dir.join("arch/arm64/configs")) {
        for e in rd.flatten().take(400) {
            search.push(e.path());
        }
    }
    for f in search {
        if let Ok(content) = std::fs::read_to_string(&f) {
            for line in content.lines() {
                let t = line.trim();
                if t.starts_with("CONFIG_KSU")
                    && !t.contains("is not set")
                    && !st.config_flags.contains(&t.to_string())
                {
                    st.config_flags.push(t.to_string());
                    st.integrated = true;
                }
            }
        }
    }

    for v in [
        "drivers/kernelsu/ksu.h",
        "fs/kernelsu/ksu.h",
        "kernel/kernelsu/ksu.h",
    ] {
        let p = kernel_dir.join(v);
        if let Ok(c) = std::fs::read_to_string(&p) {
            if let Some(line) = c
                .lines()
                .find(|l| l.contains("KERNEL_SU_VERSION") || l.contains("KSU_VERSION"))
            {
                st.version = line.trim().to_string();
                break;
            }
        }
    }

    st.provider = detect_provider(kernel_dir);
    st.message = if st.integrated {
        format!(
            "已集成 KernelSU（{}），开关 {} 项",
            if st.version.is_empty() { "版本未知" } else { &st.version },
            st.config_flags.len()
        )
    } else {
        "当前内核源码中未发现 KernelSU".into()
    };
    st
}

/// 通过特征文件猜测已集成的是哪个分支
fn detect_provider(kernel_dir: &Path) -> String {
    let checks: [(&str, &str); 7] = [
        ("drivers/kernelsu/susfs.c", "SukiSU-Ultra / BakaSU（含 SUSFS）"),
        ("fs/susfs.c", "SUSFS 已 patched"),
        ("drivers/kernelsu/Kconfig", "KernelSU 系"),
        ("drivers/kernelsu", "KernelSU 系"),
        ("fs/kernelsu", "KernelSU（fs 布局）"),
        ("kernel/kernelsu", "KernelSU（kernel 布局）"),
        ("drivers/kernelsu/mountify", "xxKSU / mountify"),
    ];
    for (p, label) in checks {
        if kernel_dir.join(p).exists() {
            return label.to_string();
        }
    }
    String::new()
}

/* --------------------------- 管理器 APK --------------------------- */

/// 解析 releases JSON 中的 .apk 资产
fn parse_release_assets(v: &serde_json::Value) -> Vec<ReleaseAsset> {
    let mut out = Vec::new();
    if let Some(arr) = v.as_array() {
        for rel in arr {
            let tag = rel.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
            if let Some(assets) = rel.get("assets").and_then(|a| a.as_array()) {
                for a in assets {
                    let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.ends_with(".apk") {
                        out.push(ReleaseAsset {
                            id: a.get("id").and_then(|i| i.as_u64()).unwrap_or(0),
                            name: name.to_string(),
                            size: a.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
                            download_url: a
                                .get("browser_download_url")
                                .and_then(|u| u.as_str())
                                .unwrap_or("")
                                .to_string(),
                            tag: tag.to_string(),
                        });
                    }
                }
            }
        }
    }
    out
}

pub async fn manager_releases(
    provider_id: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<ReleaseAsset>, String> {
    let p = find_provider(provider_id).ok_or_else(|| "未知的 KSU 来源".to_string())?;
    let (owner, repo) = mirror::normalize_repo(&p.manager_repo)
        .or_else(|| mirror::normalize_repo(&format!("{}/{}", p.owner, p.repo)))
        .ok_or_else(|| "管理器仓库地址无效".to_string())?;

    let url = format!("{api_base}/repos/{owner}/{repo}/releases?per_page=20");
    let v = get_json(&url, token).await?;
    Ok(parse_release_assets(&v))
}

/// 按 owner/repo 列举 Manager 发布（前端手动指定仓库时使用）
pub async fn list_manager_assets(
    repo: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<ReleaseAsset>, String> {
    let (owner, r) = mirror::normalize_repo(repo)
        .ok_or_else(|| "管理器仓库地址无效，应为 owner/repo".to_string())?;
    let url = format!("{api_base}/repos/{owner}/{r}/releases?per_page=20");
    let v = get_json(&url, token).await?;
    Ok(parse_release_assets(&v))
}

pub async fn download_manager(
    app: &AppHandle,
    cfg: &AppConfig,
    asset: &ReleaseAsset,
) -> Result<String, String> {
    let dir = PathBuf::from(&cfg.general.workspace_dir).join("manager");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(&asset.name);
    download_to_file(app, &cfg.mirror, &asset.download_url, &dest, &asset.name, "ksu").await?;
    Ok(dest.to_string_lossy().to_string())
}
