//! 全局状态：当前打开的 vault 上下文。

use crate::index::engine::IndexEngine;
use crate::index::watcher::WatcherHandle;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct VaultContext {
    pub root: PathBuf,
    pub engine: IndexEngine,
    pub cache_path: PathBuf,
    pub watcher: Option<WatcherHandle>,
    /// 只读资源回退根：仅临时文件模式有值，指向原始 md 所在目录。
    /// 临时模式下 md 被拷进 config_dir/temp 且以该目录为库根，原目录里的
    /// 图片/附件不在库内，resolveImageSrc 找不到就会 404。这里记下原目录，
    /// 让 list_all_files_with_mtime 与 serve_asset 在库内查不到时回退到它。
    /// 只用于读取，不参与任何写操作（原文件必须不受影响）。
    pub asset_root: Option<PathBuf>,
}

impl VaultContext {
    /// 相对路径 → 绝对路径（拒绝 `..`、绝对路径注入，保证不出 vault）
    pub fn safe_join(&self, rel: &str) -> anyhow::Result<PathBuf> {
        let rel_path = Path::new(rel);
        if rel_path.is_absolute() {
            anyhow::bail!("非法路径：{:?}", rel);
        }
        for c in rel_path.components() {
            if matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                anyhow::bail!("非法路径：{:?}", rel);
            }
        }
        Ok(self.root.join(rel_path))
    }
}

#[derive(Default)]
pub struct AppState {
    pub vault: Mutex<Option<VaultContext>>,
    /// 系统启动参数里带的文件路径（右键"用 EasyMD 打开"传入） */
    pub pending_file: Mutex<Option<String>>,
    /// 前端是否已注册好 os-open-file 监听（由 get_pending_file 在挂载完成后置位）。
    /// macOS 的 RunEvent::Opened 可能早于前端挂载，早于此刻的路径必须走 pending_file 排队，
    /// 否则 emit 出去没有接收方，文件就打不开了。
    pub frontend_ready: AtomicBool,
}
