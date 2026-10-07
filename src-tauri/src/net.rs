//! 网络层：带进度事件与镜像回退的下载器 + 归档解压 + GitHub API

use std::path::Path;

use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::log::{log_error, log_info, log_warn, PROGRESS_EVENT};
use crate::mirror;
use crate::model::{MirrorConfig, ProgressEvent};

const UA: &str = "KernelBuilderManager/0.1 (+tauri; linux)";

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(UA)
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败：{e}"))
}

fn emit_progress(app: &AppHandle, ev: ProgressEvent) {
    let _ = app.emit(PROGRESS_EVENT, ev);
}

/// 依次尝试候选地址下载；全部失败返回最后一个错误
pub async fn download_any(
    app: &AppHandle,
    id: &str,
    label: &str,
    urls: &[String],
    dest: &Path,
) -> Result<u64, String> {
    let mut last_err = String::from("没有可用的下载地址");
    for (idx, url) in urls.iter().enumerate() {
        log_info(
            app,
            "download",
            &format!("[{}/{}] 下载 {label}：{url}", idx + 1, urls.len()),
        );
        match download_one(app, id, label, url, dest).await {
            Ok(size) => return Ok(size),
            Err(e) => {
                log_warn(app, "download", &format!("下载失败：{e}"));
                last_err = e;
            }
        }
    }
    emit_progress(
        app,
        ProgressEvent {
            id: id.to_string(),
            label: label.to_string(),
            received: 0,
            total: 0,
            percent: 0.0,
            done: true,
            error: Some(last_err.clone()),
        },
    );
    log_error(app, "download", &format!("{label} 下载失败：{last_err}"));
    Err(last_err)
}

/// 高层封装：自动按镜像配置展开候选地址
pub async fn download_to_file(
    app: &AppHandle,
    mirror: &MirrorConfig,
    url: &str,
    dest: &Path,
    label: &str,
    task: &str,
) -> Result<u64, String> {
    let urls = mirror::candidates(url, mirror);
    download_any(app, task, label, &urls, dest).await
}

pub async fn download_one(
    app: &AppHandle,
    id: &str,
    label: &str,
    url: &str,
    dest: &Path,
) -> Result<u64, String> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("无法创建目录 {}：{e}", parent.display()))?;
    }

    let resp = client()?
        .get(url)
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("无法写入 {}：{e}", dest.display()))?;

    let mut received: u64 = 0;
    let mut last_percent = -1.0f64;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("传输中断：{e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("写入失败：{e}"))?;
        received += chunk.len() as u64;

        let percent = if total > 0 {
            (received as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        if (percent - last_percent).abs() >= 1.0 {
            last_percent = percent;
            emit_progress(
                app,
                ProgressEvent {
                    id: id.to_string(),
                    label: label.to_string(),
                    received,
                    total,
                    percent,
                    done: false,
                    error: None,
                },
            );
        }
    }
    file.flush().await.map_err(|e| format!("刷新文件失败：{e}"))?;
    drop(file);

    emit_progress(
        app,
        ProgressEvent {
            id: id.to_string(),
            label: label.to_string(),
            received,
            total: if total > 0 { total } else { received },
            percent: 100.0,
            done: true,
            error: None,
        },
    );
    log_info(
        app,
        "download",
        &format!("{label} 下载完成（{}）", human_size(received)),
    );
    Ok(received)
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < UNITS.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{:.1} {}", size, UNITS[i])
}

/* ----------------------------- 解压 ----------------------------- */

/// 解压 .tar.gz（strip=0 时等价于普通解压）
pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), String> {
    extract_tar_gz_strip(archive, dest, 0)
}

/// 解压 .tar.gz，并剥除前 `strip` 层目录（用于 AnyKernel3 等顶层目录）
pub fn extract_tar_gz_strip(
    archive: &Path,
    dest: &Path,
    strip: usize,
) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("创建目标目录失败：{e}"))?;
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("无法打开归档 {}：{e}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut ar = tar::Archive::new(gz);
    if strip == 0 {
        ar.unpack(dest).map_err(|e| format!("解压失败：{e}"))?;
        return Ok(());
    }
    use std::io::Read;
    for entry in ar.entries().map_err(|e| format!("读取归档失败：{e}"))? {
        let mut entry = entry.map_err(|e| format!("读取条目失败：{e}"))?;
        let path = entry
            .path()
            .map_err(|e| format!("条目路径失败：{e}"))?
            .into_owned();
        let stripped: PathBuf = path.components().skip(strip).collect();
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(&stripped);
        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&out).ok();
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut f = std::fs::File::create(&out)
            .map_err(|e| format!("创建文件失败 {}：{e}", out.display()))?;
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("读取条目内容失败：{e}"))?;
        f.write_all(&buf)
            .map_err(|e| format!("写入文件失败：{e}"))?;
    }
    Ok(())
}

/* --------------------------- GitHub API --------------------------- */

pub async fn get_json(url: &str, token: &str) -> Result<serde_json::Value, String> {
    let mut req = client()?.get(url).header("Accept", "application/vnd.github+json");
    if !token.is_empty() {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await.map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("读取响应失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("GitHub API {}：{}", status, truncate(&text, 300)));
    }
    serde_json::from_str(&text).map_err(|e| format!("解析 JSON 失败：{e}"))
}

pub async fn post_json(url: &str, token: &str, body: &serde_json::Value) -> Result<String, String> {
    if token.is_empty() {
        return Err("未配置 GitHub Token，无法调用需要鉴权的接口".into());
    }
    let resp = client()?
        .post(url)
        .header("Accept", "application/vnd.github+json")
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .map_err(|e| format!("请求失败：{e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("读取响应失败：{e}"))?;
    if !status.is_success() {
        return Err(format!("GitHub API {}：{}", status, truncate(&text, 300)));
    }
    Ok(text)
}

pub async fn get_text(url: &str, token: &str) -> Result<String, String> {
    let mut req = client()?.get(url);
    if !token.is_empty() {
        req = req.bearer_auth(token);
    }
    let resp = req.send().await.map_err(|e| format!("请求失败：{e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("读取失败：{e}"))
}

pub fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    s.chars().take(n).collect::<String>() + "…"
}

/* ----------------------------- 文件 ----------------------------- */

/// 递归收集指定后缀的文件（跳过 .git）
pub fn collect_files(root: &Path, exts: &[&str]) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    walk(root, exts, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, exts: &[&str], out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                continue;
            }
            walk(&path, exts, out);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if exts.iter().any(|e| name.ends_with(e)) {
                out.push(path);
            }
        }
    }
}

/// 读取文件前 n 行
pub fn head_lines(path: &Path, n: usize) -> String {
    std::fs::read_to_string(path)
        .map(|t| t.lines().take(n).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}
