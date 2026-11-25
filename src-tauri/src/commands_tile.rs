//! 磁贴与 MCP 状态的 Tauri 命令层：前端 ipc.ts 的 tile_* / mcp_status 命令入口。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};

use crate::mcp::{McpStatus, McpAuth};
use crate::tile::{tile_open as open_tile, SqliteTileStore, Tile, TileInput, TileStore};

/// 磁贴仓库托管状态。
pub struct TileStoreState(pub Arc<SqliteTileStore>);

/// MCP 状态托管状态。
pub struct McpState(pub Arc<Mutex<Option<McpStatus>>>);

impl McpState {
    pub fn new() -> Self { Self(Arc::new(Mutex::new(None))) }
    pub fn set(&self, status: McpStatus) -> Result<(), String> {
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        *guard = Some(status);
        Ok(())
    }
}

/// 供测试与命令层读取当前 MCP 状态值。
pub fn mcp_status_value(state: &McpState) -> Result<McpStatus, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.clone().unwrap_or(McpStatus { running: false, port: 0 }))
}

/// 将选定端口写入 mcp-port 文件（纯数字文本）。
pub fn write_mcp_port_file(path: &PathBuf, port: u16) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or_else(|| "无效 mcp-port 路径".to_string())?).map_err(|e| e.to_string())?;
    std::fs::write(path, port.to_string()).map_err(|e| e.to_string())
}

/// 从 mcp-port 文件读取端口。
pub fn read_mcp_port_file(path: &PathBuf) -> Result<u16, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    content.trim().parse::<u16>().map_err(|e| e.to_string())
}

const TILES_CHANGED_EVENT: &str = "tiles-changed";
/// MCP 服务运行时句柄：必须在 App 生命周期内持有，防止 server 线程被 drop。
pub struct McpRuntime(pub Mutex<Option<Box<dyn FnOnce() + Send>>>);
impl Drop for McpRuntime {
    fn drop(&mut self) {
        if let Some(closer) = self.0.lock().ok().and_then(|mut g| g.take()) { closer(); }
    }
}

#[tauri::command]
pub fn tile_list(state: State<'_, TileStoreState>) -> Result<Vec<Tile>, String> {
    state.0.list()
}

#[tauri::command]
pub fn tile_create(app: AppHandle, state: State<'_, TileStoreState>, input: TileInput) -> Result<Tile, String> {
    let tile = state.0.create(input)?;
    let _ = app.emit(TILES_CHANGED_EVENT, &tile);
    Ok(tile)
}

#[tauri::command]
pub fn tile_delete(app: AppHandle, state: State<'_, TileStoreState>, id: String) -> Result<(), String> {
    state.0.delete(&id)?;
    let _ = app.emit(TILES_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub fn tile_reorder(app: AppHandle, state: State<'_, TileStoreState>, id: String, order: i64) -> Result<(), String> {
    state.0.reorder(&id, order)?;
    let _ = app.emit(TILES_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub fn tile_open(state: State<'_, TileStoreState>, id: String) -> Result<(), String> {
    open_tile(&*state.0, &id)
}

#[tauri::command]
pub fn mcp_status(state: State<'_, McpState>) -> Result<McpStatus, String> {
    mcp_status_value(&state)
}

/// 应用数据目录下创建真实 SQLite store（Todo/Tile 共用 WAL 数据目录）。
pub fn open_stores(app_data_dir: &PathBuf) -> Result<(Arc<SqliteTileStore>, Arc<crate::todo::store::SqliteTodoStore>), String> {
    std::fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
    let tile = Arc::new(SqliteTileStore::open(&app_data_dir.join("tiles.db"))?);
    let todo = Arc::new(crate::todo::store::SqliteTodoStore::open(&app_data_dir.join("workbench.db")).map_err(|e| e.to_string())?);
    Ok((tile, todo))
}

/// 从本地 token 文件读取 token（不存在则生成，权限 600），不写仓库。
pub fn load_or_create_token(token_path: &PathBuf) -> Result<String, String> {
    use std::io::Write;
    #[cfg(unix)] use std::os::unix::fs::PermissionsExt;
    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if let Ok(content) = std::fs::read_to_string(token_path) {
        let trimmed = content.trim();
        if !trimmed.is_empty() { return Ok(trimmed.to_string()); }
    }
    let token = uuid::Uuid::new_v4().to_string() + &uuid::Uuid::new_v4().to_string();
    let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(token_path)
        .map_err(|e| e.to_string())?;
    file.write_all(token.as_bytes()).map_err(|e| e.to_string())?;
    #[cfg(unix)] std::fs::set_permissions(token_path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    Ok(token)
}

/// 构造鉴权器（供 lib.rs 启动 MCP 服务使用）。
pub fn auth_from_token(token: String) -> McpAuth { McpAuth::new(token) }
