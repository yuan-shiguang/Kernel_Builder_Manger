//! 内核构建：生成 build.sh 并流式执行，支持取消 / AnyKernel3 打包

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tauri::AppHandle;

use crate::log::{log_error, log_info, log_success, log_warn};
use crate::model::{BuildConfig, ToolchainPlan};
use crate::proc::{cpu_count, run_streamed};
use crate::mirror::codeload_url;
use crate::net::{download_any, extract_tar_gz_strip};

/// 生成可复现的构建脚本
pub fn generate_script(
    kernel_dir: &Path,
    plan: &ToolchainPlan,
    cfg: &BuildConfig,
) -> String {
    let clang_bin = plan
        .clang
        .as_ref()
        .map(|c| format!("{}/bin", c.path))
        .unwrap_or_default();
    let gcc64 = plan
        .gcc_aarch64
        .as_ref()
        .map(|g| format!("{}/bin/aarch64-linux-android-", g.path))
        .unwrap_or_else(|| "aarch64-linux-android-".into());
    let gcc32 = plan
        .gcc_arm32
        .as_ref()
        .map(|g| format!("{}/bin/arm-linux-androideabi-", g.path))
        .unwrap_or_else(|| "arm-linux-androideabi-".into());

    let jobs = if cfg.jobs == 0 { cpu_count() } else { cfg.jobs };
    let cc = if cfg.use_ccache {
        "ccache clang".to_string()
    } else {
        "clang".to_string()
    };
    let lto = if cfg.use_lto {
        "\nexport LTO=thin\n".to_string()
    } else {
        String::new()
    };

    let defconfig = if cfg.defconfig.is_empty() {
        "".to_string()
    } else {
        cfg.defconfig.clone()
    };

    let targets = match cfg.target.as_str() {
        "Image.gz-dtb" => "Image.gz-dtb dtbo.img",
        "dtb" => "dtbs",
        "all" => "Image Image.gz dtbs modules",
        "Image.gz" => "Image.gz",
        _ => "Image",
    };

    format!(
        r#"#!/usr/bin/env bash
# 由 Kernel Builder Manager 自动生成
set -o pipefail
cd "{kernel}"

export ARCH={arch}
export SUBARCH={arch}
export CLANG_PATH="{clang_bin}"
export PATH="$CLANG_PATH:$PATH"

export CC="{cc}"
export LD=ld.lld
export AR=llvm-ar
export NM=llvm-nm
export OBJCOPY=llvm-objcopy
export OBJDUMP=llvm-objdump
export STRIP=llvm-strip
export READELF=llvm-readelf
export LLVM=1
export LLVM_IAS=1
export CLANG_TRIPLE=aarch64-linux-gnu-
export CROSS_COMPILE="{gcc64}"
export CROSS_COMPILE_ARM32="{gcc32}"
export KBUILD_BUILD_HOST=KernelBuilderManager
export KBUILD_BUILD_USER=build
{lto}
OUT="{out}"
mkdir -p "$OUT"

echo "==> 生成配置：$OUT/.config"
make O="$OUT" ARCH={arch} {defconfig}
if [ $? -ne 0 ]; then echo "defconfig 生成失败"; exit 1; fi

# 若源码已集成 KernelSU / SUSFS，确保开关打开
if [ -f "$OUT/.config" ]; then
  for k in CONFIG_KSU CONFIG_KERNELSU CONFIG_KSU_SUSFS CONFIG_SUSFS; do
    if grep -q "^# $k is not set" "$OUT/.config"; then
      echo "==> 启用 $k"
      scripts/config --file "$OUT/.config" -e "${{k#CONFIG_}}"
    fi
  done
  make O="$OUT" ARCH={arch} olddefconfig
fi

echo "==> 开始编译（-j{jobs}）"
make -j{jobs} O="$OUT" ARCH={arch} {targets} {extra}
CODE=$?
if [ $CODE -ne 0 ]; then
  echo "构建失败，退出码 $CODE"
  exit $CODE
fi

echo "==> 构建完成，产物目录：$OUT/arch/{arch}/boot"
ls -lh "$OUT/arch/{arch}/boot" 2>/dev/null || true
"#,
        kernel = kernel_dir.display(),
        arch = cfg.arch,
        clang_bin = clang_bin,
        cc = cc,
        gcc64 = gcc64,
        gcc32 = gcc32,
        lto = lto,
        out = cfg.out_dir,
        defconfig = defconfig,
        targets = targets,
        extra = cfg.extra_args,
        jobs = jobs,
    )
}

/// 执行构建
pub async fn start(
    app: &AppHandle,
    kernel_dir: &Path,
    workspace: &Path,
    plan: &ToolchainPlan,
    cfg: &BuildConfig,
) -> Result<(i32, String), String> {
    if !kernel_dir.exists() {
        return Err(format!("内核目录不存在：{}", kernel_dir.display()));
    }
    if cfg.defconfig.is_empty() {
        return Err("未选择 defconfig，请先在「配置」页选择".into());
    }
    if let Some(clang) = &plan.clang {
        if !clang.installed {
            return Err(format!(
                "Clang 未安装：{}\n请先在「工具链」页下载。",
                clang.name
            ));
        }
    }

    let script_path = workspace.join("build.sh");
    let script = generate_script(kernel_dir, plan, cfg);
    std::fs::write(&script_path, script).map_err(|e| format!("写入 build.sh 失败：{e}"))?;
    log_info(app, "build", &format!("构建脚本已生成：{}", script_path.display()));

    let mut envs: HashMap<String, String> = HashMap::new();
    envs.insert("KBUILD_BUILD_HOST".into(), "KernelBuilderManager".into());

    let args = vec![script_path.to_string_lossy().to_string()];
    let (code, out) = run_streamed(app, "build", "bash", &args, Some(kernel_dir), &envs)?;

    if code == 0 && cfg.package_anykernel {
        match package_anykernel(app, kernel_dir, workspace, cfg).await {
            Ok(p) => log_success(app, "build", &format!("刷机包已生成：{}", p.display())),
            Err(e) => log_warn(app, "build", &format!("AnyKernel3 打包失败：{e}")),
        }
    }

    if code == 0 {
        log_success(app, "build", "内核构建完成");
    } else {
        log_error(app, "build", "内核构建失败，请查看下方错误高亮行");
    }
    Ok((code, out))
}

/// AnyKernel3 打包（构建成功后单独调用，或在 build_start 内自动触发）
pub async fn package_anykernel(
    app: &AppHandle,
    kernel_dir: &Path,
    workspace: &Path,
    cfg: &BuildConfig,
) -> Result<PathBuf, String> {
    let ak_dir = workspace.join("AnyKernel3");
    let tmp = workspace.join("AnyKernel3.tar.gz");

    if !ak_dir.exists() {
        let urls = vec![codeload_url(&cfg.anykernel_repo, "master")];
        log_info(app, "build", "下载 AnyKernel3 …");
        download_any(app, "ak3", "AnyKernel3", &urls, &tmp).await?;
        std::fs::create_dir_all(&ak_dir).ok();
        if extract_tar_gz_strip(&tmp, &ak_dir, 1).is_err() {
            let _ = std::fs::remove_dir_all(&ak_dir);
            std::fs::create_dir_all(&ak_dir).ok();
            extract_tar_gz_strip(&tmp, &ak_dir, 0)?;
        }
        let _ = std::fs::remove_file(&tmp);
    }

    let boot = kernel_dir
        .join(&cfg.out_dir)
        .join("arch")
        .join(&cfg.arch)
        .join("boot");
    if !boot.exists() {
        return Err(format!("未找到产物目录：{}", boot.display()));
    }

    for f in ["Image", "Image.gz", "Image.gz-dtb", "dtb.img", "dtbo.img"] {
        let src = boot.join(f);
        if src.exists() {
            std::fs::copy(&src, ak_dir.join(f)).ok();
            log_info(app, "build", &format!("已复制 {f}"));
        }
    }

    let envs = HashMap::new();
    let zip_name = "KernelBuilderManager.zip";
    let args = vec![
        "-r".into(),
        zip_name.into(),
        ".".into(),
        "-x".into(),
        "*.git*".into(),
    ];
    if crate::proc::which("zip") {
        let (code, _) = run_streamed(app, "build", "zip", &args, Some(&ak_dir), &envs)?;
        if code != 0 {
            return Err("zip 打包失败".into());
        }
        Ok(ak_dir.join(zip_name))
    } else {
        Err("未安装 zip，无法打包（也可手动压缩 AnyKernel3 目录）".into())
    }
}

/// 产物目录（供前端「打开目录」用）
pub fn output_dir(kernel_dir: &Path, cfg: &BuildConfig) -> PathBuf {
    kernel_dir
        .join(&cfg.out_dir)
        .join("arch")
        .join(&cfg.arch)
        .join("boot")
}

/// 扫描产物目录，返回 (文件名, 绝对路径, 字节数)
pub fn find_artifacts(kernel_dir: &Path, cfg: &BuildConfig) -> Vec<(String, String, u64)> {
    let dir = output_dir(kernel_dir, cfg);
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String, u64)> = rd
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // 只关心可刷机 / 可签名的产物
            let interesting = name.starts_with("Image")
                || name.starts_with("zImage")
                || name.ends_with(".dtb")
                || name.ends_with(".dtbo")
                || name.ends_with(".img");
            if !interesting {
                return None;
            }
            let size = e.metadata().ok().map(|m| m.len()).unwrap_or(0);
            Some((name, e.path().to_string_lossy().to_string(), size))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// 清理构建产物
pub fn clean(app: &AppHandle, kernel_dir: &Path, cfg: &BuildConfig) -> Result<(), String> {
    let out = kernel_dir.join(&cfg.out_dir);
    if !out.exists() {
        log_warn(app, "build", "out 目录不存在，无需清理");
        return Ok(());
    }
    std::fs::remove_dir_all(&out).map_err(|e| format!("清理失败：{e}"))?;
    log_success(app, "build", &format!("已清理 {}", out.display()));
    Ok(())
}
