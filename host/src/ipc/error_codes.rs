/// JSON-RPC 2.0 标准错误码 + BetterCPT 自定义错误码
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum IpcError {
    /// -32700: JSON 解析错误
    ParseError = -32700,
    /// -32600: 不符合 JSON-RPC 规范
    InvalidRequest = -32600,
    /// -32601: 方法名不存在
    MethodNotFound = -32601,
    /// -32602: 参数类型或必填字段错误
    InvalidParams = -32602,
    /// -32603: 未预期的内部错误
    InternalError = -32603,
    /// -32000: 权限不足
    PermissionDenied = -32000,
    /// -32001: Widget id 不存在
    WidgetNotFound = -32001,
    /// -32002: Runtime 不可用
    RuntimeNotReady = -32002,
    /// -32003: 操作超时
    Timeout = -32003,
}

impl IpcError {
    #[allow(dead_code)]
    pub fn message(self) -> &'static str {
        match self {
            IpcError::ParseError => "Parse error",
            IpcError::InvalidRequest => "Invalid Request",
            IpcError::MethodNotFound => "Method not found",
            IpcError::InvalidParams => "Invalid params",
            IpcError::InternalError => "Internal error",
            IpcError::PermissionDenied => "Permission denied",
            IpcError::WidgetNotFound => "Widget not found",
            IpcError::RuntimeNotReady => "Runtime not ready",
            IpcError::Timeout => "Timeout",
        }
    }
}
