use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};

use crate::runtime::capability::CapabilityRegistry;
use crate::runtime::isolate_pool::IsolatePool;
use crate::runtime::manifest::{parse_manifest, Manifest};
use crate::runtime::permissions::check_ceilings;
use crate::runtime::registry::RuntimeRegistry;

/// 插件加载器：目录扫描、安装/加载/卸载 + 热更新文件追踪
pub struct PluginLoader {
    builtin_dir: PathBuf,
    user_dir: PathBuf,
    /// 热更新追踪：plugin_id → (entry_path, last_mtime)
    watch_list: HashMap<String, (PathBuf, SystemTime)>,
}

impl PluginLoader {
    pub fn new() -> Result<Self> {
        let builtin_dir = resolve_builtin_plugin_dir();
        let user_dir = get_user_plugin_dir()?;

        fs::create_dir_all(&user_dir).context("failed to create user plugin directory")?;

        tracing::info!("Builtin plugin dir: {}", builtin_dir.display());
        Ok(Self {
            builtin_dir,
            user_dir,
            watch_list: HashMap::new(),
        })
    }

    /// 扫描两个插件目录，返回所有可用插件的 manifest 列表
    pub fn scan_plugins(&self) -> Result<Vec<Manifest>> {
        let mut manifests = vec![];

        for dir in &[&self.builtin_dir, &self.user_dir] {
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(dir).with_context(|| format!("failed to read dir: {}", dir.display()))? {
                let entry = entry?;
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    match parse_manifest(&manifest_path) {
                        Ok(m) => manifests.push(m),
                        Err(e) => {
                            tracing::warn!("Skipping invalid plugin in {}: {}", entry.path().display(), e);
                        }
                    }
                }
            }
        }

        tracing::info!("Scanned plugins: {} found", manifests.len());
        Ok(manifests)
    }

    /// 安装插件：从源路径复制到用户插件目录
    pub fn install(&mut self, source_path: &str) -> Result<Manifest> {
        let src = Path::new(source_path);
        let manifest = if src.join("manifest.json").exists() {
            // 目录形式
            parse_manifest(&src.join("manifest.json"))?
        } else if src.extension().map_or(false, |ext| ext == "bcpkg") {
            // .bcpkg 格式 — Phase 2 仅校验 manifest，不解压
            anyhow::bail!(".bcpkg extraction not yet implemented in Phase 2");
        } else {
            anyhow::bail!("invalid plugin source: {}", source_path);
        };

        let dest = self.user_dir.join(
            manifest.id.strip_prefix('@').unwrap_or(&manifest.id).replace('/', "-"),
        );
        if !dest.exists() {
            fs::create_dir_all(&dest)?;
            // 简单复制：manifest.json + entry
            fs::copy(src.join("manifest.json"), dest.join("manifest.json"))?;
            let entry_src = src.join(&manifest.entry);
            if entry_src.exists() {
                fs::copy(&entry_src, dest.join(&manifest.entry))?;
            }
        }

        tracing::info!("Plugin installed: {} -> {}", manifest.id, dest.display());
        Ok(manifest)
    }

    /// 加载插件：解析 manifest → 权限检查 → 创建 Isolate → 执行入口
    pub fn load(
        &mut self,
        plugin_id: &str,
        isolate_pool: &mut IsolatePool,
        cap_registry: &CapabilityRegistry,
        runtime_registry: &mut RuntimeRegistry,
    ) -> Result<()> {
        // 查找 manifest
        let manifest = self.find_manifest(plugin_id)?;

        // 权限检查
        check_ceilings(&manifest.runtime, &manifest.capabilities, &manifest.permissions)?;

        // 验证声明的 capabilities 均已注册
        for cap in &manifest.capabilities {
            if !cap_registry.has(cap) {
                anyhow::bail!("capability '{}' not registered in CapabilityRegistry", cap);
            }
        }

        // 读取入口代码
        let plugin_dir = self.find_plugin_dir(plugin_id)?;
        let entry_path = plugin_dir.join(&manifest.entry);
        let code = fs::read_to_string(&entry_path)
            .with_context(|| format!("failed to read entry: {}", entry_path.display()))?;

        // 注册到 RuntimeRegistry
        use crate::runtime::registry::RuntimeEntry;
        runtime_registry.register(RuntimeEntry {
            id: plugin_id.into(),
            runtime_type: manifest.runtime.clone(),
            status: "starting".into(),
        })?;

        // 创建 Isolate 执行
        isolate_pool
            .spawn(plugin_id, &code)
            .with_context(|| format!("failed to spawn isolate for '{}'", plugin_id))?;

        runtime_registry.update_status(plugin_id, "running")?;

        // 记录热更新追踪
        if let Ok(meta) = fs::metadata(&entry_path) {
            if let Ok(mtime) = meta.modified() {
                self.watch_list
                    .insert(plugin_id.to_string(), (entry_path, mtime));
            }
        }

        tracing::info!("Plugin loaded: {}", plugin_id);
        Ok(())
    }

    /// 热更新检测：扫描所有已加载插件的入口文件 mtime，返回需要重载的 (plugin_id, new_code) 列表
    pub fn check_hot_reload(&mut self) -> Vec<(String, String)> {
        let mut changed = vec![];
        let mut updated_times: Vec<(String, SystemTime)> = vec![];

        for (plugin_id, (entry_path, last_mtime)) in &self.watch_list {
            if let Ok(meta) = fs::metadata(entry_path) {
                if let Ok(mtime) = meta.modified() {
                    if mtime > *last_mtime {
                        if let Ok(code) = fs::read_to_string(entry_path) {
                            tracing::info!(
                                "Hot reload detected: {} ({} → {})",
                                plugin_id,
                                entry_path.display(),
                                code.len()
                            );
                            changed.push((plugin_id.clone(), code));
                            updated_times.push((plugin_id.clone(), mtime));
                        }
                    }
                }
            }
        }

        // 更新记录的 mtime
        for (id, time) in updated_times {
            if let Some(entry) = self.watch_list.get_mut(&id) {
                entry.1 = time;
            }
        }

        changed
    }

    /// 卸载插件：停止 Isolate → 删除文件
    #[allow(dead_code)]
    pub fn uninstall(
        &mut self,
        plugin_id: &str,
        isolate_pool: &mut IsolatePool,
        runtime_registry: &mut RuntimeRegistry,
    ) -> Result<()> {
        // 停止 Isolate
        if isolate_pool.has(plugin_id) {
            isolate_pool.kill(plugin_id).ok();
        }

        // 注销
        runtime_registry.unregister(plugin_id).ok();

        // 删除文件
        if let Ok(dir) = self.find_plugin_dir(plugin_id) {
            if dir.starts_with(&self.user_dir) {
                // 只删除用户安装的插件
                fs::remove_dir_all(&dir).ok();
                tracing::info!("Plugin uninstalled: {}", plugin_id);
            } else {
                tracing::info!("Builtin plugin '{}' files preserved", plugin_id);
            }
        }

        Ok(())
    }

    /// 获取插件的入口代码（用于 Start 重建 Isolate）
    pub fn get_plugin_code(&self, plugin_id: &str) -> Result<String> {
        let manifest = self.find_manifest(plugin_id)?;
        let plugin_dir = self.find_plugin_dir(plugin_id)?;
        let entry_path = plugin_dir.join(&manifest.entry);
        fs::read_to_string(&entry_path)
            .with_context(|| format!("failed to read entry: {}", entry_path.display()))
    }

    fn find_manifest(&self, plugin_id: &str) -> Result<Manifest> {
        for dir in &[&self.builtin_dir, &self.user_dir] {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    let m = parse_manifest(&manifest_path)?;
                    if m.id == plugin_id {
                        return Ok(m);
                    }
                }
            }
        }
        anyhow::bail!("plugin '{}' not found", plugin_id)
    }

    fn find_plugin_dir(&self, plugin_id: &str) -> Result<PathBuf> {
        for dir in &[&self.builtin_dir, &self.user_dir] {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let manifest_path = entry.path().join("manifest.json");
                if manifest_path.exists() {
                    if let Ok(m) = parse_manifest(&manifest_path) {
                        if m.id == plugin_id {
                            return Ok(entry.path());
                        }
                    }
                }
            }
        }
        anyhow::bail!("plugin dir for '{}' not found", plugin_id)
    }
}

fn resolve_builtin_plugin_dir() -> PathBuf {
    // 策略：按优先级尝试多个路径，找到第一个存在的即返回
    let candidates: Vec<PathBuf> = if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // 1) 生产安装：plugins/ 与 exe 同目录
            let prod = exe_dir.join("plugins");
            // 2) 开发构建：exe 在 host/target/debug/ → 往上 3 层到项目根/plugins/
            let dev = exe_dir.join("..").join("..").join("..").join("plugins");
            vec![prod, dev]
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    for path in &candidates {
        if path.exists() {
            tracing::info!("Builtin plugin dir resolved: {}", path.display());
            return path.clone();
        }
    }

    // 3) CWD fallback（cargo run 时 CWD 在 host/）
    let cwd = PathBuf::from("..").join("plugins");
    if cwd.exists() {
        tracing::info!("Builtin plugin dir (CWD fallback): {}", cwd.display());
        return cwd;
    }

    PathBuf::from("plugins")
}

fn get_user_plugin_dir() -> Result<PathBuf> {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Ok(PathBuf::from(appdata).join("BetterCPT").join("plugins"));
    }
    Ok(PathBuf::from("plugins"))
}
