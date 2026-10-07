//! 内核源码识别：Linux 版本解析、架构、defconfig 清单与读写

use std::path::{Path, PathBuf};

use crate::model::KernelInfo;

/// 从顶层 Makefile 解析 VERSION / PATCHLEVEL / SUBLEVEL
pub fn detect(dir: &Path) -> KernelInfo {
    let mut info = KernelInfo {
        path: dir.to_string_lossy().to_string(),
        ..Default::default()
    };

    if dir.as_os_str().is_empty() || !dir.exists() {
        info.message = "目录不存在".into();
        return info;
    }

    let makefile = dir.join("Makefile");
    if !makefile.exists() {
        info.message = "未找到顶层 Makefile，可能不是内核源码目录".into();
        return info;
    }
    info.has_makefile = true;

    let content = std::fs::read_to_string(&makefile).unwrap_or_default();
    let head: String = content.lines().take(40).collect::<Vec<_>>().join("\n");

    info.version_num = find_num(&head, "VERSION");
    info.patchlevel_num = find_num(&head, "PATCHLEVEL");
    info.sublevel_num = find_num(&head, "SUBLEVEL");

    if info.version_num > 0 && info.patchlevel_num > 0 {
        info.version = format!("{}.{}", info.version_num, info.patchlevel_num);
        info.full_version = if info.sublevel_num > 0 {
            format!(
                "{}.{}.{}",
                info.version_num, info.patchlevel_num, info.sublevel_num
            )
        } else {
            info.version.clone()
        };
    }

    info.archs = list_archs(dir);
    info.defconfigs = list_defconfigs(dir, "arm64");
    info.valid = !info.version.is_empty();

    info.message = if info.valid {
        format!(
            "Linux {} · 架构 {} 个 · defconfig {} 个",
            info.full_version,
            info.archs.len(),
            info.defconfigs.len()
        )
    } else {
        "未能从 Makefile 解析出内核版本".into()
    };
    info
}

fn find_num(text: &str, key: &str) -> u32 {
    for line in text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(key) else { continue };
        let rest = rest.trim_start();
        if !rest.starts_with('=') && !rest.starts_with(':') {
            continue;
        }
        let value = rest[1..].split('#').next().unwrap_or("").trim();
        if let Ok(n) = value.parse::<u32>() {
            return n;
        }
    }
    0
}

/// "4.9" / "5.10.120" → (5, 10)
pub fn version_tuple(v: &str) -> Option<(u32, u32)> {
    let mut it = v.split('.');
    let a = it.next()?.parse::<u32>().ok()?;
    let b = it.next()?.parse::<u32>().ok()?;
    Some((a, b))
}

pub fn list_archs(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir.join("arch")) else {
        return vec!["arm64".into()];
    };
    let mut out: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|n| matches!(n.as_str(), "arm64" | "arm" | "x86" | "x86_64" | "riscv"))
        .collect();
    out.sort();
    if out.is_empty() {
        out.push("arm64".into());
    }
    out
}

/// 列出 arch/<arch>/configs 下的 defconfig
pub fn list_defconfigs(dir: &Path, arch: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir.join("arch").join(arch).join("configs")) else {
        return Vec::new();
    };
    let mut out: Vec<String> = rd
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .filter(|n| n.ends_with("defconfig"))
        .collect();
    out.sort();
    out
}

pub fn defconfig_path(dir: &Path, arch: &str, name: &str) -> PathBuf {
    if name.contains('/') {
        dir.join(name)
    } else {
        dir.join("arch").join(arch).join("configs").join(name)
    }
}

/// 读取 defconfig 原文（保留注释与排布，便于在 UI 中直接编辑）
pub fn read_defconfig_text(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))
}

/// 按 arch/name 读取 defconfig 原文
pub fn read_defconfig(dir: &Path, arch: &str, name: &str) -> Result<String, String> {
    let path = defconfig_path(dir, arch, name);
    if !path.exists() {
        return Err(format!("defconfig 不存在：{}", path.display()));
    }
    read_defconfig_text(&path)
}

/// 写回 defconfig 原文（首次修改自动备份 *.bak）
pub fn write_defconfig(dir: &Path, arch: &str, name: &str, content: &str) -> Result<(), String> {
    let path = defconfig_path(dir, arch, name);
    if !path.exists() {
        return Err(format!("defconfig 不存在：{}", path.display()));
    }
    let bak = path.with_extension("defconfig.bak");
    if !bak.exists() {
        let _ = std::fs::copy(&path, &bak);
    }
    std::fs::write(&path, content)
        .map_err(|e| format!("写入 {} 失败：{e}", path.display()))
}

/// 读取并解析为 (key, value) —— 供需要做结构化处理的场景使用
pub fn read_defconfig_entries(path: &Path) -> Result<Vec<(String, String)>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    Ok(parse_config(&text))
}

pub fn parse_config(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().trim_matches('"').to_string()))
        })
        .collect()
}

pub fn serialize_config(items: &[(String, String)]) -> String {
    items
        .iter()
        .map(|(k, v)| {
            let quoted =
                !matches!(v.as_str(), "y" | "m" | "n") && !v.chars().all(|c| c.is_ascii_digit());
            if quoted {
                format!("{k}=\"{v}\"")
            } else {
                format!("{k}={v}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}
