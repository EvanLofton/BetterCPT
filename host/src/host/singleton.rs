use anyhow::{Context, Result};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex};

const MUTEX_NAME: &str = "BetterCPT_Host_Instance";

/// 持有 Windows 命名 Mutex 的 RAII guard，Drop 时自动释放
pub struct SingletonGuard {
    handle: HANDLE,
}

impl Drop for SingletonGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.handle);
            let _ = CloseHandle(self.handle);
        }
        tracing::info!("Singleton mutex released");
    }
}

/// 尝试获取单例锁。若已有实例运行则返回错误
pub fn acquire_singleton() -> Result<SingletonGuard> {
    let name_wide: Vec<u16> = MUTEX_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let handle = unsafe { CreateMutexW(None, true, PCWSTR::from_raw(name_wide.as_ptr())) }
        .context("failed to create singleton mutex")?;

    if handle.is_invalid() {
        anyhow::bail!("CreateMutexW returned invalid handle");
    }

    unsafe {
        if windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            anyhow::bail!("Another BetterCPT instance is already running");
        }
    }

    tracing::info!("Singleton mutex acquired");
    Ok(SingletonGuard { handle })
}
