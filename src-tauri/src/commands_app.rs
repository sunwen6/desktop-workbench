//! 应用枚举命令：扫描 macOS 已安装应用，供前端"点选"登记 App 磁贴。
//!
//! 只读、无副作用；不依赖 Tauri 运行时，纯函数便于单测。

use serde::Serialize;
use std::fs;
use std::path::Path;

/// 单个可选应用的展示信息（bundle id + 应用名）。
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AppEntry {
    /// CFBundleIdentifier，作为磁贴 target 传给 /usr/bin/open -b
    pub bundle_id: String,
    /// 应用显示名（优先 CFBundleName，回退 bundle 目录名）
    pub name: String,
}

/// 扫描目录下所有 .app，读出 (bundle_id, name)。
/// 读不到 CFBundleIdentifier 的应用跳过（无法用 bundle id 打开）。
pub fn scan_apps_in_dir(dir: &Path) -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "app"))
        .filter_map(|e| read_app_entry(&e.path()))
        .collect();
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// 读取单个 .app 的 bundle id 与显示名；缺 CFBundleIdentifier 返回 None。
fn read_app_entry(app_path: &Path) -> Option<AppEntry> {
    let plist = app_path.join("Contents/Info.plist");
    let bundle_id = read_plist_string(&plist, "CFBundleIdentifier")?;
    let name = read_plist_string(&plist, "CFBundleName").unwrap_or_else(|| {
        app_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    });
    // bundle id 非空才登记，避免空 target
    if bundle_id.trim().is_empty() {
        return None;
    }
    Some(AppEntry { bundle_id, name })
}

/// 从 Info.plist 里取单个字符串键。
/// 用 macOS 自带 `defaults read`，免去解析二进制/JSON plist 的额外依赖。
fn read_plist_string(plist_path: &Path, key: &str) -> Option<String> {
    let out = std::process::Command::new("/usr/bin/defaults")
        .arg("read")
        .arg(plist_path)
        .arg(key)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// macOS 默认应用目录。拆成函数便于测试注入。
fn default_app_dirs() -> Vec<std::path::PathBuf> {
    vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ]
}

use std::path::PathBuf;

/// 汇总默认目录的应用列表（/Applications + /System/Applications，按 bundle id 去重）。
pub fn list_apps_impl() -> Vec<AppEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut all = Vec::new();
    for dir in default_app_dirs() {
        for app in scan_apps_in_dir(&dir) {
            if seen.insert(app.bundle_id.clone()) {
                all.push(app);
            }
        }
    }
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    all
}

/// Tauri 命令封装：返回已安装应用列表。
#[tauri::command]
pub fn app_list() -> Vec<AppEntry> {
    list_apps_impl()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_real_applications_dir_finds_apps() {
        // 集成式冒烟：本机 /Applications 下应能读到至少一个带 bundle id 的应用
        let apps = scan_apps_in_dir(Path::new("/Applications"));
        assert!(!apps.is_empty(), "本机 /Applications 应存在可读应用");
        assert!(apps.iter().all(|a| !a.bundle_id.is_empty() && !a.name.is_empty()));
    }

    #[test]
    fn scan_missing_dir_returns_empty() {
        let apps = scan_apps_in_dir(Path::new("/nonexistent/path/xyz"));
        assert!(apps.is_empty());
    }
}
