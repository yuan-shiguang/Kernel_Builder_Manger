//! 工具链：内核版本 → (Clang, GCC) 映射、缺失检测、下载安装
//!
//! 版本对应关系（需求给定）：
//!   AOSP Clang r365631c —— 兜底，兼容性最好
//!   Clang 9             ↔ GCC 4.4 / 4.9
//!   Clang 11            ↔ GCC 4.14
//!   Clang 12 (r383902)  ↔ GCC 4.19 / 5.4
//!   Clang 14 (r416183b) ↔ GCC 5.10
//!   Clang 17 (r450784e) ↔ GCC 5.15 / 6.1
//!   Clang 17 (r487747c) ↔ GCC 5.15 / 6.1

use std::path::PathBuf;

use tauri::AppHandle;

use crate::kernel::version_tuple;
use crate::log::{log_info, log_success, log_warn};
use crate::mirror;
use crate::model::{AppConfig, HostDependency, ToolchainItem, ToolchainPlan};
use crate::net::{download_to_file, extract_tar_gz};
use crate::proc::{run_capture, which};

const GOOGLE: &str = "https://android.googlesource.com";

/* ----------------------------- 定义表 ----------------------------- */

pub struct ClangDef {
    pub id: &'static str,
    pub name: &'static str,
    pub tag: &'static str,
    pub note: &'static str,
    pub kernels: &'static [&'static str],
}

pub const CLANGS: &[ClangDef] = &[
    ClangDef {
        id: "clang-r365631c",
        name: "AOSP Clang r365631c",
        tag: "clang-r365631c",
        note: "通用兜底，兼容性最好",
        kernels: &[],
    },
    ClangDef {
        id: "clang-r353983c",
        name: "Clang 9",
        tag: "clang-r353983c",
        note: "搭配 GCC 4.4 / 4.9",
        kernels: &["4.4", "4.9"],
    },
    ClangDef {
        id: "clang-r399163b1",
        name: "Clang 11",
        tag: "clang-r399163b1",
        note: "搭配 GCC 4.14",
        kernels: &["4.14"],
    },
    ClangDef {
        id: "clang-r383902",
        name: "Clang 12 (r383902)",
        tag: "clang-r383902",
        note: "搭配 GCC 4.19 / 5.4",
        kernels: &["4.19", "5.4"],
    },
    ClangDef {
        id: "clang-r416183b",
        name: "Clang 14 (r416183b)",
        tag: "clang-r416183b",
        note: "搭配 GCC 5.10",
        kernels: &["5.10"],
    },
    ClangDef {
        id: "clang-r450784e",
        name: "Clang 17 (r450784e)",
        tag: "clang-r450784e",
        note: "搭配 GCC 5.15 / 6.1",
        kernels: &["5.15", "6.1"],
    },
    ClangDef {
        id: "clang-r487747c",
        name: "Clang 17 (r487747c)",
        tag: "clang-r487747c",
        note: "较新的 Clang 17，搭配 GCC 5.15 / 6.1",
        kernels: &[],
    },
];

pub const GCC_VERSIONS: &[&str] = &["4.4", "4.9", "4.14", "4.19", "5.4", "5.10", "5.15", "6.1"];

fn clang_url(tag: &str, branch: &str) -> String {
    format!("{GOOGLE}/platform/prebuilts/clang/host/linux-x86/+archive/refs/heads/{branch}/{tag}.tar.gz")
}

fn gcc_aarch64_url(v: &str) -> String {
    format!("{GOOGLE}/platform/prebuilts/gcc/linux-x86/aarch64/aarch64-linux-android-{v}/+archive/refs/heads/main.tar.gz")
}

fn gcc_arm32_url(v: &str) -> String {
    format!("{GOOGLE}/platform/prebuilts/gcc/linux-x86/arm/arm-linux-androideabi-{v}/+archive/refs/heads/main.tar.gz")
}

/* ----------------------------- 匹配 ----------------------------- */

pub fn clang_for_kernel(version: &str) -> &'static ClangDef {
    let v = version.trim();
    for c in CLANGS {
        if c.kernels.iter().any(|k| *k == v) {
            return c;
        }
    }
    if let Some((major, minor)) = version_tuple(v) {
        let pick = match (major, minor) {
            (6, _) => Some("clang-r487747c"),
            (5, m) if m >= 15 => Some("clang-r450784e"),
            (5, m) if m >= 10 => Some("clang-r416183b"),
            (5, _) => Some("clang-r383902"),
            (4, m) if m >= 19 => Some("clang-r383902"),
            (4, m) if m >= 14 => Some("clang-r399163b1"),
            (4, _) => Some("clang-r353983c"),
            _ => None,
        };
        if let Some(id) = pick {
            if let Some(c) = CLANGS.iter().find(|c| c.id == id) {
                return c;
            }
        }
    }
    &CLANGS[0]
}

pub fn gcc_for_kernel(version: &str) -> &'static str {
    let v = version.trim();
    if let Some(g) = GCC_VERSIONS.iter().find(|g| **g == v) {
        return g;
    }
    if let Some((major, minor)) = version_tuple(v) {
        // 取不超过目标版本的最近一档
        let mut best: Option<(u32, u32, &'static str)> = None;
        for g in GCC_VERSIONS {
            let Some((a, b)) = version_tuple(g) else { continue };
            if (a, b) <= (major, minor) {
                match best {
                    Some(cur) if (a, b) > (cur.0, cur.1) => best = Some((a, b, g)),
                    None => best = Some((a, b, g)),
                    _ => {}
                }
            }
        }
        if let Some(b) = best {
            return b.2;
        }
    }
    "4.9"
}

/* ----------------------------- 状态 ----------------------------- */

pub fn install_root(cfg: &AppConfig) -> PathBuf {
    PathBuf::from(&cfg.toolchain.install_dir)
}

pub fn make_clang_item(cfg: &AppConfig, def: &ClangDef) -> ToolchainItem {
    let dir = install_root(cfg).join(def.id);
    ToolchainItem {
        id: def.id.to_string(),
        name: def.name.to_string(),
        kind: "clang".into(),
        arch: "host".into(),
        version: def.tag.to_string(),
        url: mirror::googlesource(
            &clang_url(def.tag, &cfg.toolchain.clang_branch),
            &cfg.mirror.googlesource_mirror,
        ),
        github_fallback: format!("LineageOS/android_prebuilts_clang_host_linux-x86_{}", def.tag),
        dir_name: def.id.to_string(),
        installed: dir.join("bin").join("clang").exists(),
        path: dir.to_string_lossy().to_string(),
        size_hint: "约 1.2 GB".into(),
        required: def.id == "clang-r365631c",
        note: def.note.to_string(),
    }
}

pub fn make_gcc_item(cfg: &AppConfig, version: &str, arch: &str) -> ToolchainItem {
    let id = format!("gcc-{arch}-{version}");
    let dir = install_root(cfg).join(&id);
    let (url, probe) = if arch == "arm32" {
        (gcc_arm32_url(version), "bin/arm-linux-androideabi-gcc")
    } else {
        (gcc_aarch64_url(version), "bin/aarch64-linux-android-gcc")
    };
    ToolchainItem {
        id: id.clone(),
        name: if arch == "arm32" {
            format!("GCC {version} · arm32")
        } else {
            format!("GCC {version} · aarch64")
        },
        kind: "gcc".into(),
        arch: arch.to_string(),
        version: version.to_string(),
        url: mirror::googlesource(&url, &cfg.mirror.googlesource_mirror),
        github_fallback: if arch == "arm32" {
            format!("LineageOS/android_prebuilts_gcc_linux-x86_arm_arm-linux-androideabi-{version}")
        } else {
            format!("LineageOS/android_prebuilts_gcc_linux-x86_aarch64_aarch64-linux-android-{version}")
        },
        dir_name: id,
        installed: dir.join(probe).exists(),
        path: dir.to_string_lossy().to_string(),
        size_hint: "约 300 MB".into(),
        required: version == "4.9" && arch == "aarch64",
        note: format!("内核 {} 推荐搭配", version),
    }
}

pub fn list_all(cfg: &AppConfig) -> Vec<ToolchainItem> {
    let mut out = Vec::new();
    for c in CLANGS {
        out.push(make_clang_item(cfg, c));
    }
    for g in GCC_VERSIONS {
        out.push(make_gcc_item(cfg, g, "aarch64"));
        out.push(make_gcc_item(cfg, g, "arm32"));
    }
    out
}

pub fn plan(cfg: &AppConfig, kernel_version: &str) -> ToolchainPlan {
    if kernel_version.is_empty() {
        return ToolchainPlan {
            kernel_version: String::new(),
            matched: false,
            reason: "尚未识别内核版本，请先在「源码」页选择内核目录".into(),
            clang: None,
            gcc_aarch64: None,
            gcc_arm32: None,
            missing: vec![],
        };
    }

    let clang_id = if cfg.toolchain.clang_override.is_empty() {
        clang_for_kernel(kernel_version).id.to_string()
    } else {
        cfg.toolchain.clang_override.clone()
    };
    let gcc_ver = if cfg.toolchain.gcc_override.is_empty() {
        gcc_for_kernel(kernel_version).to_string()
    } else {
        cfg.toolchain.gcc_override.clone()
    };

    let clang = CLANGS
        .iter()
        .find(|c| c.id == clang_id)
        .map(|c| make_clang_item(cfg, c));
    let gcc_aarch64 = Some(make_gcc_item(cfg, &gcc_ver, "aarch64"));
    let gcc_arm32 = Some(make_gcc_item(cfg, &gcc_ver, "arm32"));

    let mut missing = Vec::new();
    for item in [&clang, &gcc_aarch64, &gcc_arm32].into_iter().flatten() {
        if !item.installed {
            missing.push(item.clone());
        }
    }

    ToolchainPlan {
        kernel_version: kernel_version.to_string(),
        matched: true,
        reason: format!(
            "内核 {} → {} + GCC {}",
            kernel_version,
            clang.as_ref().map(|c| c.name.clone()).unwrap_or_default(),
            gcc_ver
        ),
        clang,
        gcc_aarch64,
        gcc_arm32,
        missing,
    }
}

/* ----------------------------- 安装 ----------------------------- */

pub async fn install(app: &AppHandle, cfg: &AppConfig, item: &ToolchainItem) -> Result<String, String> {
    let root = install_root(cfg);
    let dest = root.join(&item.dir_name);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let tmp = root.join(format!("{}.tar.gz", item.dir_name));

    log_info(app, "toolchain", &format!("开始下载 {}", item.name));

    if let Err(e) = download_to_file(app, &cfg.mirror, &item.url, &tmp, &item.name, "toolchain").await {
        log_warn(app, "toolchain", &format!("官方源失败（{e}），改用 GitHub 镜像仓库"));
        let Some((owner, repo)) = item.github_fallback.split_once('/') else {
            return Err(format!("{e}；且缺少 GitHub 兜底仓库"));
        };
        let url = mirror::codeload_url(&format!("{owner}/{repo}"), "main");
        download_to_file(app, &cfg.mirror, &url, &tmp, &item.name, "toolchain").await?;
    }

    log_info(app, "toolchain", &format!("解压 {} …", item.name));
    extract_tar_gz(&tmp, &dest)?;
    let _ = std::fs::remove_file(&tmp);
    let _ = run_capture(
        "sh",
        &["-c", &format!("chmod -R +x '{}' 2>/dev/null || true", dest.display())],
    );

    log_success(app, "toolchain", &format!("{} 就绪 → {}", item.name, dest.display()));
    Ok(dest.to_string_lossy().to_string())
}

/// 首次初始化必备组件
pub fn bootstrap_items(cfg: &AppConfig) -> Vec<ToolchainItem> {
    list_all(cfg).into_iter().filter(|i| i.required).collect()
}

pub fn verify(item: &ToolchainItem) -> Result<String, String> {
    let dir = PathBuf::from(&item.path);
    if !dir.exists() {
        return Err(format!("{} 未安装", item.name));
    }
    let exe = if item.kind == "clang" {
        dir.join("bin").join("clang")
    } else if item.arch == "arm32" {
        dir.join("bin").join("arm-linux-androideabi-gcc")
    } else {
        dir.join("bin").join("aarch64-linux-android-gcc")
    };
    if !exe.exists() {
        return Err(format!("缺少可执行文件 {}", exe.display()));
    }
    let Some((code, out, err)) = run_capture(exe.to_string_lossy().as_ref(), &["--version"]) else {
        return Err("执行失败".into());
    };
    if code != 0 {
        return Err(err);
    }
    Ok(out.lines().next().unwrap_or("").to_string())
}

/* --------------------------- 宿主依赖 --------------------------- */

pub fn host_dependencies() -> Vec<HostDependency> {
    let required = [
        ("git", "拉取源码"),
        ("curl", "下载工具链"),
        ("tar", "解压工具链"),
        ("make", "内核构建"),
        ("bc", "内核脚本"),
        ("bison", "内核脚本"),
        ("flex", "内核脚本"),
        ("python3", "内核脚本"),
        ("openssl", "模块签名"),
        ("cpio", "打包 ramdisk"),
        ("unzip", "解压 Actions 产物"),
    ];
    let optional = [
        ("ccache", "加速重复编译"),
        ("lz4", "压缩镜像"),
        ("dtc", "设备树编译"),
        ("rsync", "文件同步"),
        ("clang", "宿主 clang（可选）"),
        ("zip", "打包刷机包"),
    ];

    let mut out = Vec::new();
    for (n, note) in required {
        out.push(HostDependency {
            name: n.into(),
            present: which(n),
            required: true,
            note: note.into(),
        });
    }
    for (n, note) in optional {
        out.push(HostDependency {
            name: n.into(),
            present: which(n),
            required: false,
            note: note.into(),
        });
    }
    out
}

pub fn install_command() -> String {
    let pkgs = "git curl tar make bc bison flex python3 openssl cpio ccache lz4 unzip zip rsync";
    if which("apt-get") {
        format!("sudo apt-get update && sudo apt-get install -y {pkgs} device-tree-compiler")
    } else if which("dnf") {
        format!("sudo dnf install -y {pkgs} dtc")
    } else if which("pacman") {
        format!("sudo pacman -S --needed {pkgs} dtc")
    } else if which("zypper") {
        format!("sudo zypper install -y {pkgs} dtc")
    } else {
        format!("请手动安装：{pkgs}")
    }
}
