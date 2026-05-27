use anyhow::Result;
use tokio::sync::broadcast;

/// 退出信号监听器：等待托盘"退出"或 Ctrl-C
pub struct ShutdownListener {
    rx: broadcast::Receiver<()>,
}

impl ShutdownListener {
    /// 创建监听器并注册 Ctrl-C handler
    pub fn new(tx: broadcast::Sender<()>) -> Result<Self> {
        let rx = tx.subscribe();

        // 注册 Ctrl-C handler（tokio 信号）
        let ctrlc_tx = tx;
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                tracing::info!("Ctrl-C received, broadcasting shutdown");
                let _ = ctrlc_tx.send(());
            }
        });

        Ok(Self { rx })
    }

    /// 阻塞等待退出信号（托盘退出 或 Ctrl-C）
    #[allow(dead_code)]
    pub async fn wait(mut self) -> Result<()> {
        self.recv().await
    }

    /// 非消耗版本，适合 tokio::select! 循环
    pub async fn recv(&mut self) -> Result<()> {
        match self.rx.recv().await {
            Ok(()) => {
                tracing::info!("Shutdown signal received");
                Ok(())
            }
            Err(e) => {
                tracing::info!("All shutdown senders dropped ({}), exiting", e);
                Ok(())
            }
        }
    }
}
