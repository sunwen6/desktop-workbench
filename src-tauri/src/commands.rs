//! 前端命令层：把 Task 1 的 TodoStore 包装成 Tauri 命令，供 React 前端经 IPC 调用。
//!
//! 设计：核心逻辑写成纯函数 `todo_*_impl`（入参是 `&dyn TodoStore`，不依赖 Tauri 运行时），
//! 便于在没有 Tauri App 的测试环境直接单测；`#[tauri::command]` 只是薄封装，从托管状态取 store 后转发。
//! 任何写操作成功后 emit `todos-changed` 事件，前端据此自动刷新。

use tauri::{AppHandle, Emitter, Manager, State};

use crate::todo::{
    CreateTodoInput, StoreError, Todo, TodoFilter, TodoStore, UpdateTodoPatch,
};

/// todos-changed 事件名：写操作完成后通知前端刷新（前端 lib/ipc.ts 中的 TODO_CHANGED_EVENT）
pub const TODO_CHANGED_EVENT: &str = "todos-changed";

/// 托管状态：共享同一个 TodoStore（UI 与 MCP 共用，避免双写不一致）
pub struct StoreState(pub std::sync::Arc<dyn TodoStore>);

// ---- 核心逻辑（无 Tauri 运行时依赖，可直接单测） ----

pub fn todo_create_impl(
    store: &dyn TodoStore,
    input: CreateTodoInput,
) -> Result<Todo, StoreError> {
    store.create(input)
}

pub fn todo_list_impl(store: &dyn TodoStore, filter: TodoFilter) -> Result<Vec<Todo>, StoreError> {
    store.list(filter)
}

pub fn todo_complete_impl(store: &dyn TodoStore, id: String) -> Result<Todo, StoreError> {
    store.complete(&id)
}

pub fn todo_delete_impl(store: &dyn TodoStore, id: String) -> Result<(), StoreError> {
    store.delete(&id)
}

pub fn todo_update_impl(
    store: &dyn TodoStore,
    id: String,
    patch: UpdateTodoPatch,
) -> Result<Todo, StoreError> {
    store.update(&id, patch)
}

// ---- Tauri 命令封装（供前端 invoke） ----

#[tauri::command]
pub fn todo_create(
    app: AppHandle,
    state: State<StoreState>,
    input: CreateTodoInput,
) -> Result<Todo, String> {
    let todo = todo_create_impl(&*state.0, input).map_err(|e| e.to_string())?;
    let _ = app.emit(TODO_CHANGED_EVENT, &todo);
    Ok(todo)
}

#[tauri::command]
pub fn todo_list(state: State<StoreState>, filter: TodoFilter) -> Result<Vec<Todo>, String> {
    todo_list_impl(&*state.0, filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn todo_complete(
    app: AppHandle,
    state: State<StoreState>,
    id: String,
) -> Result<Todo, String> {
    let todo = todo_complete_impl(&*state.0, id).map_err(|e| e.to_string())?;
    let _ = app.emit(TODO_CHANGED_EVENT, &todo);
    Ok(todo)
}

#[tauri::command]
pub fn todo_delete(app: AppHandle, state: State<StoreState>, id: String) -> Result<(), String> {
    todo_delete_impl(&*state.0, id).map_err(|e| e.to_string())?;
    let _ = app.emit(TODO_CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub fn todo_update(
    app: AppHandle,
    state: State<StoreState>,
    id: String,
    patch: UpdateTodoPatch,
) -> Result<Todo, String> {
    let todo = todo_update_impl(&*state.0, id, patch).map_err(|e| e.to_string())?;
    let _ = app.emit(TODO_CHANGED_EVENT, &todo);
    Ok(todo)
}

// ---- 窗口控制命令 ----

/// 显示/隐藏主窗口（⌥Space 触发）
#[tauri::command]
pub fn window_toggle(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            win.hide().map_err(|e| e.to_string())?;
        } else {
            win.show().map_err(|e| e.to_string())?;
            let _ = win.set_focus();
        }
    }
    Ok(())
}

/// 显示窗口并聚焦快速记录框
#[tauri::command]
pub fn window_focus_capture(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.show().map_err(|e| e.to_string())?;
        let _ = win.set_focus();
        let _ = win.emit("focus-capture", ());
    }
    Ok(())
}

/// 置顶开关（偏好持久化由 backend 的 AppState 表负责）
#[tauri::command]
pub fn window_set_always_on_top(app: AppHandle, enabled: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        win.set_always_on_top(enabled).map_err(|e| e.to_string())?;
    }
    Ok(())
}
