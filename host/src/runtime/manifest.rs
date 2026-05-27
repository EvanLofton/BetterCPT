use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// 校验插件 ID 格式：@author/plugin-name（全小写，字母/数字/连字符）
pub fn validate_plugin_id(id: &str) -> Result<()> {
    let err = || anyhow::anyhow!("invalid plugin id '{}': must match @author/plugin-name", id);

    if !id.starts_with('@') || id.len() < 4 {
        return Err(err());
    }

    // 找到唯一一个 '/'
    let slash_pos = id.find('/').ok_or_else(err)?;
    if id[slash_pos + 1..].find('/').is_some() {
        return Err(err()); // 多个 '/'
    }

    let author = &id[1..slash_pos];
    let name = &id[slash_pos + 1..];

    if author.is_empty() || name.is_empty() {
        return Err(err());
    }

    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
    if !author.chars().all(valid) || !name.chars().all(valid) {
        return Err(err());
    }

    // 首字符必须是字母
    if !author.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(err());
    }
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(err());
    }

    Ok(())
}

/// HOST 版本号（编译时从 Cargo.toml 自动获取）
const HOST_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 解析 "X.Y.Z" → (u32, u32, u32)
fn parse_semver(s: &str) -> Result<(u32, u32, u32)> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 3 {
        anyhow::bail!("invalid semver: '{}'", s);
    }
    let major = parts[0].parse::<u32>().context("invalid major")?;
    let minor = parts[1].parse::<u32>().context("invalid minor")?;
    let patch = parts[2].parse::<u32>().context("invalid patch")?;
    Ok((major, minor, patch))
}

/// 比较两个 semver：a >= b
fn semver_gte(a: (u32, u32, u32), b: (u32, u32, u32)) -> bool {
    a.0 > b.0 || (a.0 == b.0 && a.1 > b.1) || (a.0 == b.0 && a.1 == b.1 && a.2 >= b.2)
}

/// semver 引擎版本校验：解析 host 版本与 required 范围比对
pub fn check_engine_version(required: &str) -> Result<()> {
    let required = required.trim();
    if required.is_empty() || required == "*" {
        return Ok(());
    }

    if let Some(ver_str) = required.strip_prefix(">=") {
        let host_ver = parse_semver(HOST_VERSION)?;
        let req_ver = parse_semver(ver_str)?;
        if semver_gte(host_ver, req_ver) {
            return Ok(());
        }
        anyhow::bail!(
            "host version {} does not satisfy engine requirement '{}'",
            HOST_VERSION, required
        );
    }

    anyhow::bail!("unsupported engine requirement: '{}'", required)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub runtime: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub entry: String,
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
}

/// 从路径读取并解析 manifest.json
pub fn parse_manifest(path: &Path) -> Result<Manifest> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest: {}", path.display()))?;

    let manifest: Manifest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse manifest JSON: {}", path.display()))?;

    // 校验插件 ID 格式
    validate_plugin_id(&manifest.id)?;

    // 校验 runtime 字段
    if manifest.runtime != "widget" && manifest.runtime != "script" {
        anyhow::bail!(
            "invalid runtime '{}' in plugin '{}': must be 'widget' or 'script'",
            manifest.runtime,
            manifest.id
        );
    }

    // engine 版本校验
    if !manifest.engine.is_empty() {
        check_engine_version(&manifest.engine)?;
    }

    tracing::info!(
        "Manifest loaded: id={}, runtime={}, caps={:?}, perms={:?}",
        manifest.id,
        manifest.runtime,
        manifest.capabilities,
        manifest.permissions
    );

    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_plugin_ids() {
        assert!(validate_plugin_id("@local/test").is_ok());
        assert!(validate_plugin_id("@bettercpt/dock").is_ok());
        assert!(validate_plugin_id("@a/b").is_ok());
        assert!(validate_plugin_id("@foo-bar/my-plugin-2").is_ok());
    }

    #[test]
    fn invalid_plugin_ids() {
        assert!(validate_plugin_id("no-at-sign").is_err());
        assert!(validate_plugin_id("@").is_err());
        assert!(validate_plugin_id("@/name").is_err());       // author 为空
        assert!(validate_plugin_id("@author/").is_err());      // name 为空
        assert!(validate_plugin_id("@Author/name").is_err());  // 大写
        assert!(validate_plugin_id("@author/Name").is_err());  // 大写
        assert!(validate_plugin_id("@author/n@me").is_err());  // 特殊字符
        assert!(validate_plugin_id("@author/pkg/sub").is_err()); // 多个 /
    }

    #[test]
    fn engine_version_star_or_empty() {
        assert!(check_engine_version("*").is_ok());
        assert!(check_engine_version("").is_ok());
        assert!(check_engine_version(">=0.1.0").is_ok());
    }

}
