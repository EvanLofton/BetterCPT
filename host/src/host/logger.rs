use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024; // 10MB per file

struct RollingWriter {
    dir: PathBuf,
    current_date: String,
    current_seq: u32,
    current_size: u64,
    file: Option<fs::File>,
}

impl RollingWriter {
    fn new(dir: PathBuf) -> Self {
        Self { dir, current_date: String::new(), current_seq: 0, current_size: 0, file: None }
    }

    fn rotate(&mut self) {
        let today = today_str();
        if today != self.current_date {
            self.current_date = today;
            self.current_seq = 0;
        } else {
            self.current_seq += 1;
        }
        self.current_size = 0;

        let name = if self.current_seq == 0 {
            format!("bettercpt_{}.log", self.current_date)
        } else {
            format!("bettercpt_{}.{}.log", self.current_date, self.current_seq)
        };

        let _ = fs::create_dir_all(&self.dir);
        let path = self.dir.join(&name);
        self.file = fs::OpenOptions::new().create(true).append(true).open(&path).ok();
    }
}

impl Write for RollingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.file.is_none() || self.current_size >= MAX_LOG_SIZE {
            self.rotate();
        }
        if let Some(ref mut f) = self.file {
            let n = f.write(buf)?;
            self.current_size += n as u64;
            f.flush()?;
            Ok(n)
        } else {
            std::io::stderr().write(buf)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(ref mut f) = self.file { f.flush() } else { Ok(()) }
    }
}

/// 初始化日志：stderr + 按日期和大小旋转的文件日志，存储于 exe/log/
pub fn init_logger() -> Result<()> {
    let log_dir = get_log_dir()?;
    let writer = RollingWriter::new(log_dir.clone());
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let stderr_layer = fmt::layer().with_target(false).with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_target(false).with_ansi(false).with_writer(Mutex::new(writer));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("logger already initialized: {}", e))?;

    tracing::info!("Logger initialized, dir: {}", log_dir.display());
    Ok(())
}

fn get_log_dir() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return Ok(parent.join("log"));
        }
    }
    Ok(PathBuf::from("log"))
}

fn today_str() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64 + 719468);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
