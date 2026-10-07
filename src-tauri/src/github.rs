//! GitHub Actions 客户端：工作流列举 / 触发 / 运行查询 / 产物下载

use std::path::Path;

use serde_json::json;
use tauri::AppHandle;

use crate::log::{log_error, log_info, log_success, log_warn};
use crate::model::{GhArtifact, GhRun, GhWorkflow};
use crate::net::{download_one, get_json, human_size, post_json};

/// 带 token 的 API GET（401/403 时给出可操作的提示）
pub async fn api_get(url: &str, token: &str) -> Result<serde_json::Value, String> {
    get_json(url, token).await.map_err(|e| enrich_error(&e, token))
}

fn enrich_error(e: &str, token: &str) -> String {
    if token.is_empty() && (e.contains("401") || e.contains("403") || e.contains("rate limit")) {
        return format!("{e}\n建议：在设置页填写 GitHub Token（匿名 API 有严格的速率限制）");
    }
    if e.contains("401") {
        return format!("{e}\nToken 无效或已过期，请重新生成（需要 repo / actions 权限）");
    }
    if e.contains("403") {
        return format!("{e}\nToken 权限不足，请确认勾选了 repo 与 workflow 权限");
    }
    e.to_string()
}

pub async fn list_workflows(
    owner: &str,
    repo: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<GhWorkflow>, String> {
    if owner.is_empty() || repo.is_empty() {
        return Err("未填写 owner / repo".into());
    }
    let url = format!("{api_base}/repos/{owner}/{repo}/actions/workflows?per_page=100");
    let value = api_get(&url, token).await?;
    let arr = value
        .get("workflows")
        .and_then(|w| w.as_array())
        .ok_or("返回格式异常")?;
    Ok(arr
        .iter()
        .map(|w| GhWorkflow {
            id: w.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: w
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            path: w
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            state: w
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

pub async fn dispatch_workflow(
    owner: &str,
    repo: &str,
    workflow_id: &str,
    git_ref: &str,
    inputs: &serde_json::Value,
    token: &str,
    api_base: &str,
) -> Result<String, String> {
    if token.is_empty() {
        return Err("触发工作流需要 GitHub Token（需要 workflow 权限）".into());
    }
    if workflow_id.is_empty() {
        return Err("未选择工作流".into());
    }
    let url = format!("{api_base}/repos/{owner}/{repo}/actions/workflows/{workflow_id}/dispatches");
    let body = json!({ "ref": git_ref, "inputs": inputs });
    post_json(&url, token, &body).await?;
    Ok(format!("已触发 {workflow_id} @ {git_ref}"))
}

pub async fn list_runs(
    owner: &str,
    repo: &str,
    token: &str,
    api_base: &str,
) -> Result<Vec<GhRun>, String> {
    let url = format!("{api_base}/repos/{owner}/{repo}/actions/runs?per_page=30");
    let value = api_get(&url, token).await?;
    let arr = value
        .get("workflow_runs")
        .and_then(|r| r.as_array())
        .ok_or("返回格式异常")?;
    Ok(arr
        .iter()
        .map(|r| GhRun {
            id: r.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: r
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            status: r
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            conclusion: r
                .get("conclusion")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            html_url: r
                .get("html_url")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            created_at: r
                .get("created_at")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            head_branch: r
                .get("head_branch")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            display_title: r
                .get("display_title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect())
}

pub async fn list_artifacts(
    owner: &str,
    repo: &str,
    run_id: u64,
    token: &str,
    api_base: &str,
) -> Result<Vec<GhArtifact>, String> {
    let url = format!("{api_base}/repos/{owner}/{repo}/actions/runs/{run_id}/artifacts?per_page=100");
    let value = api_get(&url, token).await?;
    let arr = value
        .get("artifacts")
        .and_then(|a| a.as_array())
        .ok_or("返回格式异常")?;
    Ok(arr
        .iter()
        .map(|a| GhArtifact {
            id: a.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
            name: a
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            size: a
                .get("size_in_bytes")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            expired: a.get("expired").and_then(|v| v.as_bool()).unwrap_or(false),
            run_id,
        })
        .collect())
}

pub async fn cancel_run(
    owner: &str,
    repo: &str,
    run_id: u64,
    token: &str,
    api_base: &str,
) -> Result<String, String> {
    if token.is_empty() {
        return Err("取消运行需要 GitHub Token".into());
    }
    let url = format!("{api_base}/repos/{owner}/{repo}/actions/runs/{run_id}/cancel");
    crate::net::post_json(&url, token, &json!({})).await?;
    Ok(format!("已请求取消运行 #{run_id}"))
}

/// 下载产物 zip（需要 token），然后用 unzip 解压
pub async fn download_artifact(
    app: &AppHandle,
    owner: &str,
    repo: &str,
    artifact: &GhArtifact,
    dest_dir: &Path,
    token: &str,
    api_base: &str,
) -> Result<String, String> {
    if token.is_empty() {
        return Err("下载产物需要 GitHub Token".into());
    }
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("创建目录失败：{e}"))?;

    let url = format!(
        "{api_base}/repos/{owner}/{repo}/actions/artifacts/{}/zip",
        artifact.id
    );
    let zip_path = dest_dir.join(format!("{}.zip", artifact.name));
    let size = human_size(artifact.size);

    log_info(
        app,
        "actions",
        &format!("下载产物 {}（{}）…", artifact.name, size),
    );

    // 该接口会 302 到临时地址，reqwest 默认跟随重定向（不带 Authorization），
    // 因此这里直接用带 token 的 GET 由 net::download_one 处理
    match download_one(app, "artifact", &artifact.name, &url, &zip_path).await {
        Ok(_) => {}
        Err(e) => {
            // 部分情况下需要手工带上 token 请求
            log_warn(app, "actions", &format!("直连失败（{e}），尝试带鉴权重试 …"));
            download_one(
                app,
                "artifact",
                &artifact.name,
                &format!("{url}?token={token}"),
                &zip_path,
            )
            .await
            .map_err(|e2| {
                let msg = format!("产物下载失败：{e2}\n（GitHub 产物接口要求 token 具备 actions:read 权限）");
                log_error(app, "actions", &msg);
                msg
            })?;
        }
    }

    let target = dest_dir.join(&artifact.name);
    std::fs::create_dir_all(&target).ok();

    if crate::proc::which("unzip") {
        let out = std::process::Command::new("unzip")
            .args(["-o", "-q"])
            .arg(&zip_path)
            .arg("-d")
            .arg(&target)
            .env("LC_ALL", "C")
            .output();
        match out {
            Ok(o) if o.status.success() => {
                log_success(app, "actions", &format!("产物已解压到 {}", target.display()));
                Ok(target.to_string_lossy().to_string())
            }
            Ok(o) => Err(format!(
                "解压失败：{}",
                String::from_utf8_lossy(&o.stderr)
            )),
            Err(e) => Err(format!("无法执行 unzip：{e}")),
        }
    } else {
        Ok(zip_path.to_string_lossy().to_string())
    }
}
