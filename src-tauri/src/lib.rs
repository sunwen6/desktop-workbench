//! Tauri 库入口：注册命令、托管状态、全局快捷键与菜单栏托盘。
//!
//! 数据真源为本地 SQLite（TodoStore / TileStore），UI 命令与 MCP 工具共用同一份数据。
//! `MemStore` 仅为测试保留的内存实现，不再作为 App 运行时状态。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

pub mod commands;
pub mod commands_app;
pub mod commands_pick;
pub mod commands_tile;
pub mod mcp;
pub mod tile;
pub mod todo;

use commands::StoreState;
use todo::{
    CreateTodoInput, StoreError, Todo, TodoFilter, TodoStore, TodoStatus, UpdateTodoPatch,
};

/// 测试用内存实现（App 运行时使用 SQLite TodoStore）
#[derive(Default)]
pub struct MemStore {
    items: Mutex<Vec<Todo>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TodoStore for MemStore {
    fn create(&self, input: CreateTodoInput) -> Result<Todo, StoreError> {
        if input.title.trim().is_empty() {
            return Err(StoreError::Validation("标题不能为空".into()));
        }
        let now = chrono::Utc::now().to_rfc3339();
        let todo = Todo {
            id: uuid::Uuid::new_v4().to_string(),
            title: input.title.trim().to_string(),
            status: input.status.unwrap_or(TodoStatus::Inbox),
            tags: input.tags.unwrap_or_default(),
            due_at: input.due_at,
            source: input.source,
            created_at: now.clone(),
            updated_at: now,
            completed_at: None,
        };
        self.items.lock().unwrap().push(todo.clone());
        Ok(todo)
    }

    fn list(&self, filter: TodoFilter) -> Result<Vec<Todo>, StoreError> {
        let items = self.items.lock().unwrap();
        let mut out: Vec<Todo> = items
            .iter()
            .filter(|t| filter.status.map_or(true, |s| t.status == s))
            .filter(|t| {
                filter
                    .tag
                    .as_ref()
                    .map_or(true, |tag| t.tags.iter().any(|x| x == tag))
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // created_at DESC
        if let Some(limit) = filter.limit {
            out.truncate(limit);
        }
        Ok(out)
    }

    fn update(&self, id: &str, patch: UpdateTodoPatch) -> Result<Todo, StoreError> {
        let mut items = self.items.lock().unwrap();
        let t = items
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or(StoreError::NotFound)?;
        if let Some(title) = patch.title {
            if title.trim().is_empty() {
                return Err(StoreError::Validation("标题不能为空".into()));
            }
            t.title = title.trim().to_string();
        }
        if let Some(status) = patch.status {
            t.status = status;
            if status == TodoStatus::Done && t.completed_at.is_none() {
                t.completed_at = Some(chrono::Utc::now().to_rfc3339());
            }
        }
        if let Some(tags) = patch.tags {
            t.tags = tags;
        }
        if let Some(due) = patch.due_at {
            t.due_at = Some(due);
        }
        t.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(t.clone())
    }

    fn complete(&self, id: &str) -> Result<Todo, StoreError> {
        self.update(
            id,
            UpdateTodoPatch {
                status: Some(TodoStatus::Done),
                ..Default::default()
            },
        )
    }

    fn delete(&self, id: &str) -> Result<(), StoreError> {
        let mut items = self.items.lock().unwrap();
        let before = items.len();
        items.retain(|t| t.id != id);
        if items.len() == before {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }
}

/// 应用数据子目录名：所有本地数据（workbench.db / tiles.db / mcp-port / mcp-token）都落在
/// ~/Library/Application Support/Workbench/ 下，不污染 Application Support 根目录。
pub const APP_DATA_SUBDIR: &str = "Workbench";

/// 在给定父目录下拼出应用数据目录（供测试注入临时父目录）。
pub fn app_data_parent_dir(base: &PathBuf) -> PathBuf {
    base.join(APP_DATA_SUBDIR)
}

/// 应用数据目录（窗口偏好持久化由 backend 的 AppState 表负责，这里只提供路径）
pub fn app_data_dir() -> PathBuf {
    app_data_parent_dir(&dirs_next::data_dir().unwrap_or_else(|| PathBuf::from(".")))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage({
            let dir = app_data_dir();
            let (_tiles, todos) = commands_tile::open_stores(&dir).expect("打开本地 SQLite 失败");
            StoreState(todos)
        })
        .manage({
            let dir = app_data_dir();
            let (tiles, _todos) = commands_tile::open_stores(&dir).expect("打开本地 SQLite 失败");
            commands_tile::TileStoreState(tiles)
        })
        .manage(commands_tile::McpState::new())
        .manage(commands_tile::McpRuntime(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::todo_create,
            commands::todo_list,
            commands::todo_complete,
            commands::todo_delete,
            commands::todo_update,
            commands::window_toggle,
            commands::window_focus_capture,
            commands::window_set_always_on_top,
            commands_tile::tile_list,
            commands_tile::tile_create,
            commands_tile::tile_delete,
            commands_tile::tile_reorder,
            commands_tile::tile_open,
            commands_tile::mcp_status,
            commands_app::app_list,
            commands_pick::pick_folder,
        ])
        .setup(|app| {
            // 注册全局快捷键 ⌥Space → window_toggle / 聚焦记录框
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            app.global_shortcut().on_shortcut("alt+space", |app, _shortcut, _event| {
                let _ = app.emit("toggle-window", ());
            })?;

            // 菜单栏托盘：点击显示/隐藏窗口
            let show = MenuItem::with_id(app, "show", "显示工作台", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            // 启动本地 MCP 服务：token -> 端口选择并绑定 -> 写入 mcp-port 与 McpState
            let dir = app_data_dir();
            let token_path = dir.join("mcp-token");
            let port_path = dir.join("mcp-port");
            match (|| -> Result<(), String> {
                let token = commands_tile::load_or_create_token(&token_path)?;
                let todos = app.state::<StoreState>().0.clone();
                let app_handle = app.app_handle().clone();
                let on_changed: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
                    let _ = app_handle.emit("todos-changed", ());
                });
                // port 传 0：由 start_mcp_server_with_emitter 在 17321-17330 内递增绑定，消除探测与绑定的竞态
                let (status, closer) = mcp::start_mcp_server_with_emitter(
                    todos, commands_tile::auth_from_token(token), "127.0.0.1", 0, on_changed,
                )?;
                commands_tile::write_mcp_port_file(&port_path, status.port)?;
                app.state::<commands_tile::McpRuntime>().0.lock().map_err(|e| e.to_string())?.replace(closer);
                app.state::<commands_tile::McpState>().set(status)?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(message) => {
                    // 启动失败时保持 McpState 为 running=false，UI 会显示未运行；不阻断桌面窗口。
                    eprintln!("[Workbench] MCP 启动失败: {message}");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
