//! 系统级打开：Windows 右键菜单 / 双击 md 文件拉起应用。
//! 启动参数中的文件路径 → 定位所属 vault（没有则以其所在文件夹建库）→ 打开笔记。

use crate::state::AppState;
use tauri::{AppHandle, State};

#[derive(serde::Serialize)]
pub struct OpenNoteResult {
    pub root: String,
    pub notes: Vec<crate::index::model::NoteIndex>,
    pub rel: String,
}

/// 取出启动时传入的待打开文件（读取后清空），并把前端标记为已就绪。
/// 前端在 onMounted 里先注册 os-open-file 监听、再调本命令，因此这里的置位
/// 之后到来的 RunEvent::Opened 一定会走 emit 通路，不会丢事件。
#[tauri::command]
pub fn get_pending_file(state: State<'_, AppState>) -> Option<String> {
    state
        .frontend_ready
        .store(true, std::sync::atomic::Ordering::Release);
    let mut pf = state.pending_file.lock().ok()?;
    pf.take()
}

/// 打开一个磁盘上的 md/canvas 文件：
/// 1) 在已知 vault 列表中找包含它的库（最长路径前缀）；
/// 2) 找不到则把它所在文件夹作为新库打开；
/// 3) 返回 vault 上下文 + 文件相对路径，前端直接展示。
#[tauri::command]
pub fn open_path_from_os(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenNoteResult, String> {
    let abs = std::path::PathBuf::from(&path);
    if !abs.is_absolute() {
        return Err("必须使用绝对路径".into());
    }
    let abs_norm = abs.to_string_lossy().replace('\\', "/");
    let abs_norm = abs_norm.trim_end_matches('/').to_string();
    let lower = abs_norm.to_lowercase();

    // 找包含该文件的 vault（最长前缀优先）
    let vaults = crate::config::load_vaults(&app);
    let mut best: Option<String> = None;
    for v in &vaults {
        let vp = v.path.replace('\\', "/");
        let vp = vp.trim_end_matches('/').to_string();
        let vpl = vp.to_lowercase();
        if lower.starts_with(&format!("{}/", vpl)) {
            match &best {
                Some(b) if b.len() >= vp.len() => {}
                _ => best = Some(vp),
            }
        }
    }

    let root_str = match best {
        Some(v) => v,
        None => {
            // 不属于任何库：以文件所在文件夹为库
            abs_norm
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_string())
                .unwrap_or_else(|| abs_norm.clone())
        }
    };

    let rel = abs_norm
        .strip_prefix(&root_str)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();

    // 归一化为 Windows 反斜杠形式：与 VaultPicker 存的路径一致，
    // 避免 touch_vault / last_vault / 缓存路径 因分隔符差异产生重复记录。
    // rel 保持正斜杠（前端索引/文件树使用正斜杠相对路径）。
    #[cfg(windows)]
    let root_str = root_str.replace('/', "\\");

    let res = super::vault::open_vault_inner(
        app,
        state.clone(),
        std::path::PathBuf::from(root_str.clone()),
        root_str,
    )?;
    Ok(OpenNoteResult {
        root: res.root,
        notes: res.notes,
        rel,
    })
}

// ---- Windows 右键菜单注册（绿色版也可用，写 HKCU 无需管理员） ----

#[cfg(windows)]
fn write_context_reg(exe: &str) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let cmd = format!("\"{}\" \"%1\"", exe);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("打开注册表失败：{}", e))?;

    for ext in ["md", "markdown", "canvas"] {
        // 1) 直接的右键菜单项："用 EasyMD 打开"
        let shell_path = format!("SystemFileAssociations\\.{}\\shell\\EasyMD", ext);
        let (key, _) = classes
            .create_subkey(&shell_path)
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        key.set_value("", &"用 EasyMD 打开")
            .map_err(|e| e.to_string())?;
        key.set_value("Icon", &exe).map_err(|e| e.to_string())?;
        let (cmd_key, _) = key
            .create_subkey("command")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        cmd_key.set_value("", &cmd).map_err(|e| e.to_string())?;

        // 2) "打开方式"候选列表
        let (prog_key, _) = classes
            .create_subkey("EasyMD.file")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        prog_key
            .set_value("", &"EasyMD 文档")
            .map_err(|e| e.to_string())?;
        let (icon_key, _) = prog_key
            .create_subkey("DefaultIcon")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        icon_key
            .set_value("", &format!("{},0", exe))
            .map_err(|e| e.to_string())?;
        let (open_key, _) = prog_key
            .create_subkey(r"shell\open\command")
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        open_key.set_value("", &cmd).map_err(|e| e.to_string())?;
        let (ext_key, _) = classes
            .create_subkey(format!(".{}\\OpenWithProgids", ext))
            .map_err(|e| format!("写入注册表失败：{}", e))?;
        ext_key
            .set_value("EasyMD.file", &"")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
fn remove_context_reg() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .map_err(|e| format!("打开注册表失败：{}", e))?;
    for ext in ["md", "markdown", "canvas"] {
        let _ =
            classes.delete_subkey_all(format!("SystemFileAssociations\\.{}\\shell\\EasyMD", ext));
        let _ = classes.delete_subkey_all(format!(".{}\\OpenWithProgids", ext));
        let _ = classes.delete_subkey_all("EasyMD.file");
    }
    Ok(())
}

/// 临时文件打开：拷贝到内置临时目录后以该目录为 vault 打开
#[tauri::command]
pub fn open_as_temp(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<OpenNoteResult, String> {
    let src = std::path::PathBuf::from(&path);
    if !src.is_absolute() || !src.exists() {
        return Err("文件不存在".into());
    }

    // 临时目录：app_config_dir/temp/
    let temp_dir = crate::config::config_dir(&app).join("temp");
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败：{}", e))?;

    // 拷贝文件
    let name = src
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dst = temp_dir.join(&name);
    std::fs::copy(&src, &dst).map_err(|e| format!("拷贝失败：{}", e))?;

    // 以临时目录为 vault 打开。md 被拷进 temp，但它引用的相对图片仍在原目录，
    // 所以把原文件所在目录记为只读资源根：资源列表与 serve_asset 在库内落空时回退到它，
    // 否则 `![](pic.png)` 解析出的 emdasset 请求会一律 404。
    let asset_root = src.parent().map(|p| p.to_path_buf());
    let root_str = temp_dir.to_string_lossy().to_string();
    let res = super::vault::open_vault_inner(
        app,
        state.clone(),
        std::path::PathBuf::from(temp_dir),
        root_str,
    )?;
    {
        let mut guard = state.vault.lock().map_err(|e| e.to_string())?;
        if let Some(vc) = guard.as_mut() {
            vc.asset_root = asset_root.filter(|p| p.is_dir());
        }
    }
    Ok(OpenNoteResult {
        root: res.root,
        notes: res.notes,
        rel: name,
    })
}

/// Open a validated HTTP(S) URL with the system default browser.
#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    open_external_url_inner(&url)
}

fn validate_external_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|_| "Invalid URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || url.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err("Only HTTP(S) URLs are allowed".into());
    }
    Ok(())
}

fn open_external_url_inner(url: &str) -> Result<(), String> {
    use std::process::Command;
    validate_external_url(url)?;
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(windows)]
    let mut command = Command::new("explorer.exe");
    #[cfg(not(any(target_os = "macos", windows)))]
    let mut command = Command::new("xdg-open");
    command
        .arg(url)
        .spawn()
        .map_err(|e| format!("Unable to open URL: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod external_url_tests {
    use super::validate_external_url;

    #[test]
    fn accepts_only_valid_http_urls() {
        assert!(validate_external_url("https://example.com/path?q=1").is_ok());
        assert!(validate_external_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_external_url("file:///etc/passwd").is_err());
        assert!(validate_external_url("javascript:alert(1)").is_err());
        assert!(validate_external_url("https://example.com/\nopen").is_err());
    }
}

/// 打开系统设置 → 默认应用（让用户设 EasyMD 为 .md 默认打开方式）
/// Windows 走 ms-settings:defaultapps；macOS 无对应 URI，直接提示用户在「打开方式」里设置。
#[tauri::command]
pub fn open_default_apps_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "ms-settings:defaultapps"])
            .spawn()
            .map_err(|e| format!("Open settings failed: {}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        // macOS：文件关联由 LaunchServices 管理，交给系统「显示简介 → 打开方式」，
        // 这里不弹系统窗口，只返回明确提示，避免静默失败。
        Err(
            "macOS 请在访达中右键 .md 文件 → 显示简介 → 打开方式，选择 EasyMD 后点「全部更改」"
                .into(),
        )
    }
}

/// 注册右键菜单（使用当前运行的 exe 路径）
#[tauri::command]
pub fn register_context_menu() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| format!("获取程序路径失败：{}", e))?;
        let exe = exe.to_string_lossy().to_string();
        write_context_reg(&exe)
    }
    #[cfg(not(windows))]
    {
        Err("右键菜单注册仅支持 Windows；macOS 的文件关联由系统 LaunchServices 管理".into())
    }
}

/// 取消注册
#[tauri::command]
pub fn unregister_context_menu() -> Result<(), String> {
    #[cfg(windows)]
    {
        remove_context_reg()
    }
    #[cfg(not(windows))]
    {
        Err("右键菜单注册仅支持 Windows".into())
    }
}
