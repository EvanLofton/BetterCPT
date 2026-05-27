use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcErrorPayload>,
    pub id: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcErrorPayload {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

// ── SDK Method 名常量（state_tree + deno_host 共享，编译期防拼写错误）──
pub const METHOD_CREATE_RECT: &str = "createRect";
pub const METHOD_CREATE_ICON: &str = "createIcon";
pub const METHOD_CREATE_TEXT: &str = "createText";
pub const METHOD_SET_SCALE: &str = "setScale";
pub const METHOD_SET_OPACITY: &str = "setOpacity";
pub const METHOD_SET_BLUR: &str = "setBlur";
pub const METHOD_ADD_CHILD: &str = "addChild";
pub const METHOD_REMOVE_CHILD: &str = "removeChild";
pub const METHOD_REMOVE_WIDGET: &str = "removeWidget";

use crate::ipc::error_codes::IpcError;

impl IpcResponse {
    #[allow(dead_code)]
    pub fn success(id: u64, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    #[allow(dead_code)]
    pub fn error(id: u64, err: IpcError, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(IpcErrorPayload {
                code: err as i32,
                message: err.message().into(),
                data,
            }),
            id,
        }
    }
}
