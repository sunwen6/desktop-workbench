//! 待办模块聚合：backend Task 1 的 `store.rs`（SQLite 真源）与 `migrations/` 落在此目录。
//! 前端命令层（`crate::commands`）只依赖 `model.rs` 的类型与 `TodoStore` trait，
//! 不直接触碰 SQLite，便于 backend 独立实现数据层而不影响 IPC 契约。

pub mod model;
pub mod store;

pub use model::{
    CreateTodoInput, StoreError, Todo, TodoFilter, TodoStatus, TodoStore, UpdateTodoPatch,
};
