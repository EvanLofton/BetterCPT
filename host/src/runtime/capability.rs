use std::collections::HashMap;

use anyhow::Result;

/// capability 名称 → 运行时绑定描述
#[derive(Debug, Clone)]
pub struct CapabilityBinding {
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub description: String,
}

/// Capability/Permission → 实际实现的运行时绑定注册表
pub struct CapabilityRegistry {
    bindings: HashMap<String, CapabilityBinding>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            bindings: HashMap::new(),
        };
        // 预注册 v1 已知 capability
        reg.register("widget:overlay", "Widget overlay 桌面窗口渲染");
        reg.register("window:create", "创建 Utility Window（Tauri WebView）");
        reg.register("notification", "系统通知推送");
        reg
    }

    fn register(&mut self, name: &str, description: &str) {
        self.bindings.insert(
            name.into(),
            CapabilityBinding {
                name: name.into(),
                description: description.into(),
            },
        );
    }

    /// 注册新的 capability → 绑定关系
    pub fn register_capability(&mut self, name: &str, description: &str) -> Result<()> {
        tracing::info!("Capability registered: {}", name);
        self.register(name, description);
        Ok(())
    }

    /// 查询 capability 是否存在
    pub fn has(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// 获取 capability 的绑定信息
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&CapabilityBinding> {
        self.bindings.get(name)
    }

    /// 列出所有已注册的 capability
    #[allow(dead_code)]
    pub fn list(&self) -> Vec<&CapabilityBinding> {
        self.bindings.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_registered_capabilities_present() {
        let reg = CapabilityRegistry::new();
        assert!(reg.has("widget:overlay"));
        assert!(reg.has("window:create"));
        assert!(reg.has("notification"));
    }

    #[test]
    fn register_new_capability() {
        let mut reg = CapabilityRegistry::new();
        reg.register_capability("custom:test", "Test capability").unwrap();
        assert!(reg.has("custom:test"));
    }

    #[test]
    fn unknown_capability_not_present() {
        let reg = CapabilityRegistry::new();
        assert!(!reg.has("unknown:power"));
    }
}
