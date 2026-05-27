use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::ipc::messages::{
    IpcRequest, METHOD_ADD_CHILD, METHOD_CREATE_ICON, METHOD_CREATE_RECT,
    METHOD_CREATE_TEXT, METHOD_REMOVE_CHILD, METHOD_SET_BLUR, METHOD_SET_OPACITY,
    METHOD_SET_SCALE,
};

/// Widget 类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetType {
    Rect,
    Icon,
    Text,
}

impl WidgetType {
    fn from_method(method: &str) -> Option<Self> {
        match method {
            METHOD_CREATE_RECT => Some(Self::Rect),
            METHOD_CREATE_ICON => Some(Self::Icon),
            METHOD_CREATE_TEXT => Some(Self::Text),
            _ => None,
        }
    }
}

/// Widget 位置和大小
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Default for Bounds {
    fn default() -> Self {
        Self { x: 0, y: 0, w: 0, h: 0 }
    }
}

/// Widget 节点 —— Host 唯一 UI 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetNode {
    pub id: String,
    pub widget_type: WidgetType,
    pub bounds: Bounds,
    #[serde(default)]
    pub scale: f64, // 默认 1.0
    #[serde(default)]
    pub opacity: f64, // 默认 1.0
    #[serde(default)]
    pub blur: bool,
    pub children: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default)]
    pub radius: u32,
    #[serde(default)]
    pub size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
}

/// StateTree —— Host 唯一 UI 状态源，所有变更递增版本号
pub struct StateTree {
    version: u64,
    nodes: HashMap<String, WidgetNode>,
    /// 创建顺序（用于崩溃恢复时按序回放）
    creation_order: Vec<String>,
}

impl StateTree {
    pub fn new() -> Self {
        Self {
            version: 0,
            nodes: HashMap::new(),
            creation_order: Vec::new(),
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn insert(&mut self, node: WidgetNode) -> Result<()> {
        let id = node.id.clone();
        if self.nodes.contains_key(&id) {
            anyhow::bail!("Widget id {} already exists", id);
        }
        self.creation_order.push(id.clone());
        self.nodes.insert(id, node);
        self.version += 1;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update(&mut self, id: &str, node: WidgetNode) -> Result<()> {
        if !self.nodes.contains_key(id) {
            anyhow::bail!("Widget id {} not found", id);
        }
        self.nodes.insert(id.to_string(), node);
        self.version += 1;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, id: &str) -> Result<()> {
        if self.nodes.remove(id).is_none() {
            anyhow::bail!("Widget id {} not found", id);
        }
        self.creation_order.retain(|i| i != id);
        self.version += 1;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&WidgetNode> {
        self.nodes.get(id)
    }

    /// 返回所有节点，按创建顺序（用于崩溃恢复回放）
    pub fn all_nodes(&self) -> Vec<&WidgetNode> {
        self.creation_order
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// 将 IPC 命令应用到 StateTree，保持状态同步
    pub fn apply_ipc(&mut self, req: &IpcRequest) -> Result<()> {
        match req.method.as_str() {
            METHOD_CREATE_RECT | METHOD_CREATE_ICON | METHOD_CREATE_TEXT => {
                let id = req
                    .params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if id.is_empty() || self.nodes.contains_key(&id) {
                    return Ok(()); // 已存在则跳过（热更新匹配场景）
                }

                let widget_type = WidgetType::from_method(&req.method)
                    .unwrap_or(WidgetType::Rect);

                let bounds = Bounds {
                    x: req.params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    y: req.params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    w: req.params.get("w").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    h: req.params.get("h").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                };

                let node = WidgetNode {
                    id,
                    widget_type,
                    bounds,
                    scale: 1.0,
                    opacity: 1.0,
                    blur: req.params.get("blur").and_then(|v| v.as_bool()).unwrap_or(false),
                    children: vec![],
                    color: req.params.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    text: req.params.get("text").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    src: req.params.get("src").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    radius: req.params.get("radius").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    size: req.params.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    weight: req.params.get("weight").and_then(|v| v.as_str()).map(|s| s.to_string()),
                };

                self.insert(node)?;
            }
            METHOD_SET_SCALE => {
                let id = req.params.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let scale = req.params.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0);
                if let Some(node) = self.nodes.get_mut(id) {
                    node.scale = scale;
                    self.version += 1;
                }
            }
            METHOD_SET_OPACITY => {
                let id = req.params.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let opacity = req.params.get("opacity").and_then(|v| v.as_f64()).unwrap_or(1.0);
                if let Some(node) = self.nodes.get_mut(id) {
                    node.opacity = opacity;
                    self.version += 1;
                }
            }
            METHOD_SET_BLUR => {
                let id = req.params.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let enabled = req.params.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                if let Some(node) = self.nodes.get_mut(id) {
                    node.blur = enabled;
                    self.version += 1;
                }
            }
            METHOD_ADD_CHILD => {
                let parent_id = req.params.get("parentId").and_then(|v| v.as_str()).unwrap_or("");
                let child_id = req.params.get("childId").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    if !parent.children.contains(&child_id.to_string()) {
                        parent.children.push(child_id.to_string());
                        self.version += 1;
                    }
                }
            }
            METHOD_REMOVE_CHILD => {
                let parent_id = req.params.get("parentId").and_then(|v| v.as_str()).unwrap_or("");
                let child_id = req.params.get("childId").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    parent.children.retain(|c| c != child_id);
                    self.version += 1;
                }
            }
            _ => {} // raise, lower, onHover 等不影响状态
        }
        Ok(())
    }

    /// 生成崩溃恢复所需的回放命令序列。
    ///
    /// 假设：插件始终先创建父节点再 addChild。若先子后父，回放时子节点尚未创建，addChild 会失败。
    /// v1 所有插件符合此假设；若未来有反向依赖，需改为拓扑排序。
    ///
    /// 已知限制：onMouseMove 等需要 Qt 侧 setMouseTracking(true) 的事件，Replay 后不会恢复。Phase 5 处理。
    pub fn generate_replay_commands(&self) -> Vec<IpcRequest> {
        let mut commands = Vec::new();

        // 第一遍：创建所有 widget
        for node in self.all_nodes() {
            let params = match node.widget_type {
                WidgetType::Rect => {
                    serde_json::json!({
                        "id": node.id,
                        "x": node.bounds.x,
                        "y": node.bounds.y,
                        "w": node.bounds.w,
                        "h": node.bounds.h,
                        "blur": node.blur,
                        "radius": node.radius,
                        "color": node.color,
                    })
                }
                WidgetType::Icon => {
                    serde_json::json!({
                        "id": node.id,
                        "x": node.bounds.x,
                        "y": node.bounds.y,
                        "size": node.size,
                        "src": node.src,
                    })
                }
                WidgetType::Text => {
                    serde_json::json!({
                        "id": node.id,
                        "x": node.bounds.x,
                        "y": node.bounds.y,
                        "text": node.text,
                        "size": node.size,
                        "color": node.color,
                        "weight": node.weight,
                    })
                }
            };

            let method = match node.widget_type {
                WidgetType::Rect => METHOD_CREATE_RECT,
                WidgetType::Icon => METHOD_CREATE_ICON,
                WidgetType::Text => METHOD_CREATE_TEXT,
            };

            commands.push(IpcRequest {
                jsonrpc: "2.0".into(),
                method: method.into(),
                params,
                id: 0,
            });
        }

        // 第二遍：恢复属性（scale, opacity）
        for node in self.all_nodes() {
            if (node.scale - 1.0).abs() > f64::EPSILON {
                commands.push(IpcRequest {
                    jsonrpc: "2.0".into(),
                    method: METHOD_SET_SCALE.into(),
                    params: serde_json::json!({ "id": node.id, "scale": node.scale }),
                    id: 0,
                });
            }
            if (node.opacity - 1.0).abs() > f64::EPSILON {
                commands.push(IpcRequest {
                    jsonrpc: "2.0".into(),
                    method: METHOD_SET_OPACITY.into(),
                    params: serde_json::json!({ "id": node.id, "opacity": node.opacity }),
                    id: 0,
                });
            }
        }

        // 第三遍：恢复父子关系
        for node in self.all_nodes() {
            for child_id in &node.children {
                commands.push(IpcRequest {
                    jsonrpc: "2.0".into(),
                    method: METHOD_ADD_CHILD.into(),
                    params: serde_json::json!({
                        "parentId": node.id,
                        "childId": child_id,
                    }),
                    id: 0,
                });
            }
        }

        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_rect_applies_to_state() {
        let mut tree = StateTree::new();
        let cmd = IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createRect".into(),
            params: json!({"id": "widget_1", "x": 100, "y": 200, "w": 400, "h": 80, "blur": true, "radius": 16}),
            id: 1,
        };
        tree.apply_ipc(&cmd).unwrap();
        assert_eq!(tree.node_count(), 1);
        let node = tree.get("widget_1").unwrap();
        assert_eq!(node.bounds.x, 100);
        assert_eq!(node.bounds.w, 400);
        assert!(node.blur);
    }

    #[test]
    fn set_scale_updates_state() {
        let mut tree = StateTree::new();
        let create = IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createIcon".into(),
            params: json!({"id": "w1", "x": 0, "y": 0, "size": 48, "src": "test.png"}),
            id: 1,
        };
        tree.apply_ipc(&create).unwrap();

        let scale = IpcRequest {
            jsonrpc: "2.0".into(),
            method: METHOD_SET_SCALE.into(),
            params: json!({"id": "w1", "scale": 1.2}),
            id: 2,
        };
        tree.apply_ipc(&scale).unwrap();
        assert!((tree.get("w1").unwrap().scale - 1.2).abs() < 0.001);
    }

    #[test]
    fn add_child_updates_parent() {
        let mut tree = StateTree::new();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createRect".into(),
            params: json!({"id": "dock", "x": 0, "y": 0, "w": 400, "h": 80}),
            id: 1,
        }).unwrap();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createIcon".into(),
            params: json!({"id": "icon_1", "x": 16, "y": 16, "size": 48, "src": "c.png"}),
            id: 2,
        }).unwrap();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: METHOD_ADD_CHILD.into(),
            params: json!({"parentId": "dock", "childId": "icon_1"}),
            id: 3,
        }).unwrap();
        assert_eq!(tree.get("dock").unwrap().children, vec!["icon_1"]);
    }

    #[test]
    fn replay_generates_create_then_children() {
        let mut tree = StateTree::new();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createRect".into(),
            params: json!({"id": "bg", "x": 0, "y": 0, "w": 200, "h": 100, "blur": true, "radius": 8}),
            id: 1,
        }).unwrap();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: "createIcon".into(),
            params: json!({"id": "icon", "x": 10, "y": 10, "size": 32, "src": "a.png"}),
            id: 2,
        }).unwrap();
        tree.apply_ipc(&IpcRequest {
            jsonrpc: "2.0".into(),
            method: METHOD_ADD_CHILD.into(),
            params: json!({"parentId": "bg", "childId": "icon"}),
            id: 3,
        }).unwrap();

        let replay = tree.generate_replay_commands();
        // 创建顺序: bg → icon → setScale* → setOpacity* → addChild
        assert_eq!(replay[0].method, "createRect");
        assert_eq!(replay[1].method, "createIcon");
        let add_child = replay.last().unwrap();
        assert_eq!(add_child.method, METHOD_ADD_CHILD);
    }
}
