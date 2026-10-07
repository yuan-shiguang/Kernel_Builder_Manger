//! GitHub / GoogleSource 镜像源：URL 改写 + 多源回退
//!
//! 三种镜像形态：
//!   - direct   ：原样返回
//!   - prefix   ：`https://ghproxy.net/` + 原始 URL（对 github.com / codeload / raw / release 均适用）
//!   - jsdelivr ：`github.com/{o}/{r}/(blob|raw)/{ref}/{path}` → `cdn.jsdelivr.net/gh/{o}/{r}@{ref}/{path}`

use crate::model::{Mirror, MirrorConfig};

/// 按镜像规则改写单个 URL（失败时原样返回）
pub fn rewrite(url: &str, mirror: &Mirror) -> String {
    match mirror.kind.as_str() {
        "direct" => url.to_string(),
        "prefix" => {
            if mirror.prefix.is_empty() {
                url.to_string()
            } else {
                format!("{}{}", mirror.prefix.trim_end_matches('/'), url)
            }
        }
        "jsdelivr" => to_jsdelivr(url).unwrap_or_else(|| url.to_string()),
        _ => url.to_string(),
    }
}

fn to_jsdelivr(url: &str) -> Option<String> {
    let body = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    let mut parts = body.splitn(5, '/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    let kind = parts.next()?;
    let git_ref = parts.next()?;
    let path = parts.next().unwrap_or("");
    if kind != "blob" && kind != "raw" {
        return None;
    }
    Some(format!(
        "https://cdn.jsdelivr.net/gh/{owner}/{repo}@{git_ref}/{path}"
    ))
}

/// 当前启用的镜像；未开启镜像功能时返回 None
pub fn active_mirror(cfg: &MirrorConfig) -> Option<&Mirror> {
    if !cfg.enabled {
        return None;
    }
    cfg.mirrors.iter().find(|m| m.id == cfg.active_id && m.enabled)
}

/// 生成候选地址列表：[镜像后的地址, 原始地址]（auto_fallback 关闭时只给镜像地址）
pub fn candidates(url: &str, cfg: &MirrorConfig) -> Vec<String> {
    let mut list = Vec::new();
    if let Some(m) = active_mirror(cfg) {
        let rewritten = rewrite(url, m);
        if rewritten != url {
            list.push(rewritten);
        }
    }
    if cfg.auto_fallback || list.is_empty() {
        list.push(url.to_string());
    }
    list
}

/// 带来源标注的候选列表，便于日志排查
pub fn candidate_urls(url: &str, cfg: &MirrorConfig) -> Vec<(String, String)> {
    let mut list = Vec::new();
    if let Some(m) = active_mirror(cfg) {
        let rewritten = rewrite(url, m);
        if rewritten != url {
            list.push((m.id.clone(), rewritten));
        }
    }
    if cfg.auto_fallback || list.is_empty() {
        list.push(("direct".to_string(), url.to_string()));
    }
    list
}

/* ---------------------------- URL 构造 ---------------------------- */

/// GitHub 归档（tar.gz），repo 传 "owner/repo"
pub fn archive_url(repo: &str, git_ref: &str) -> String {
    format!("https://github.com/{repo}/archive/refs/heads/{git_ref}.tar.gz")
}

/// codeload 归档（部分镜像对 codeload 更友好）
pub fn codeload_url(repo: &str, git_ref: &str) -> String {
    format!("https://codeload.github.com/{repo}/tar.gz/refs/heads/{git_ref}")
}

pub fn raw_url(repo: &str, git_ref: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{repo}/{git_ref}/{path}")
}

pub fn clone_https(repo: &str) -> String {
    format!("https://github.com/{repo}.git")
}

pub fn clone_ssh(repo: &str) -> String {
    format!("git@github.com:{repo}.git")
}

/// googlesource 镜像替换
pub fn googlesource(url: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return url.to_string();
    }
    url.replace(
        "https://android.googlesource.com/",
        &format!("{}/", prefix.trim_end_matches('/')),
    )
}

/// 统一解析 owner/repo、git@github.com:o/r.git、https://github.com/o/r
pub fn normalize_repo(input: &str) -> Option<(String, String)> {
    let s = input.trim().trim_end_matches('/').trim_end_matches(".git");
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        let mut it = rest.split('/');
        let owner = it.next()?.to_string();
        let repo = it.next()?.to_string();
        if repo.is_empty() {
            return None;
        }
        return Some((owner, repo));
    }
    if let Some(rest) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
    {
        let mut it = rest.split('/');
        let owner = it.next()?.to_string();
        let repo = it.next()?.to_string();
        if repo.is_empty() {
            return None;
        }
        return Some((owner, repo));
    }
    let mut it = s.split('/');
    let owner = it.next()?.to_string();
    let repo = it.next()?.to_string();
    if owner.is_empty() || repo.is_empty() || it.next().is_some() {
        return None;
    }
    Some((owner, repo))
}
