use anyhow::Result;
use std::collections::HashSet;

/// Widget Runtime 能力上限
const WIDGET_CAPABILITIES: &[&str] = &["widget:overlay", "window:create", "notification"];

/// Script Runtime 能力上限
const SCRIPT_CAPABILITIES: &[&str] = &["window:create", "notification"];

/// Widget Runtime 权限上限（非常受限）
const WIDGET_PERMISSIONS: &[&str] = &["storage"];

/// Script Runtime 权限上限
const SCRIPT_PERMISSIONS: &[&str] = &[
    "fs:read",
    "fs:write",
    "network:fetch",
    "storage",
    "dialog:open",
    "dialog:save",
    "system:info",
];

/// 检查插件声明的 capabilities 和 permissions 是否超出 Runtime 上限
pub fn check_ceilings(
    runtime_type: &str,
    capabilities: &[String],
    permissions: &[String],
) -> Result<()> {
    let (cap_ceiling, perm_ceiling) = match runtime_type {
        "widget" => (WIDGET_CAPABILITIES, WIDGET_PERMISSIONS),
        "script" => (SCRIPT_CAPABILITIES, SCRIPT_PERMISSIONS),
        other => anyhow::bail!("unknown runtime type: {}", other),
    };

    let cap_set: HashSet<&&str> = cap_ceiling.iter().collect();
    let perm_set: HashSet<&&str> = perm_ceiling.iter().collect();

    for cap in capabilities {
        if !cap_set.contains(&cap.as_str()) {
            anyhow::bail!(
                "capability '{}' exceeds {} runtime ceiling",
                cap,
                runtime_type
            );
        }
    }

    for perm in permissions {
        if !perm_set.contains(&perm.as_str()) {
            anyhow::bail!(
                "permission '{}' exceeds {} runtime ceiling",
                perm,
                runtime_type
            );
        }
    }

    tracing::info!(
        "Permission check passed for runtime={}, caps={:?}, perms={:?}",
        runtime_type,
        capabilities,
        permissions
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_can_use_widget_overlay() {
        assert!(check_ceilings("widget", &["widget:overlay".into()], &["storage".into()]).is_ok());
    }

    #[test]
    fn widget_cannot_fs_write() {
        assert!(check_ceilings("widget", &[], &["fs:write".into()]).is_err());
    }

    #[test]
    fn widget_cannot_fs_read() {
        assert!(check_ceilings("widget", &[], &["fs:read".into()]).is_err());
    }

    #[test]
    fn script_can_fs_read_write() {
        assert!(check_ceilings(
            "script",
            &["window:create".into()],
            &["fs:read".into(), "fs:write".into(), "network:fetch".into()]
        )
        .is_ok());
    }

    #[test]
    fn script_cannot_widget_overlay() {
        assert!(check_ceilings("script", &["widget:overlay".into()], &[]).is_err());
    }

    #[test]
    fn unknown_runtime_rejected() {
        assert!(check_ceilings("native", &[], &[]).is_err());
    }
}
