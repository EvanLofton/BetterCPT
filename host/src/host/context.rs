use std::sync::Arc;

use anyhow::Result;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

use crate::ipc::messages::IpcRequest;
use crate::scheduler::RenderScheduler;

/// Host 全局上下文，持有所有核心服务的 Arc 引用。
/// 不含 !Sync 类型，可以在 Tauri 闭包之间自由传递。
pub struct AppContext {
    pub shutdown_tx: broadcast::Sender<()>,
    pub scheduler_tx: tokio::sync::mpsc::UnboundedSender<IpcRequest>,
    pub scheduler: Arc<Mutex<RenderScheduler>>,
}

impl AppContext {
    pub fn new() -> Result<Arc<Self>> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        let (scheduler_tx, scheduler) = RenderScheduler::new();

        Ok(Arc::new(Self {
            shutdown_tx,
            scheduler_tx,
            scheduler: Arc::new(Mutex::new(scheduler)),
        }))
    }
}
