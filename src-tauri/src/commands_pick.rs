//! 原生选择器命令：文件夹走系统目录选择对话框（tauri-plugin-dialog），供前端"浏览"登记 Folder 磁贴。
//!
//! 选择器在 Rust 侧调用，前端只 invoke 命令，无需前端 dialog 权限。

use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

/// 阻塞式弹出目录选择框，返回用户选中的目录路径。
/// 用户取消返回 None。
///
/// 注意：`blocking_pick_folder` 会阻塞当前线程直到对话框关闭，
/// 因此该命令注册为 async，由 Tauri 在独立线程池调度，不卡 UI。
#[tauri::command]
pub async fn pick_folder<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>, String> {
    let folder = app
        .dialog()
        .file()
        .set_title("选择文件夹")
        .blocking_pick_folder();
    // FilePath 经 simplified() 归一化，再转成路径字符串；取消返回 None
    Ok(folder.and_then(|p| p.simplified().into_path().ok()?.to_str().map(|s| s.to_string())))
}
