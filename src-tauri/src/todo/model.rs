//! 待办数据模型：与前端 `src/lib/ipc.ts` 及实施计划 Task 1 Interfaces 保持一致。
//! SQLite 真源由 backend 的 Task 1（TodoStore）提供；本文件只定义跨 IPC 传输的数据契约。

use serde::{Deserialize, Serialize};

/// 待办状态：收件箱 / 今天 / 已完成
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TodoStatus {
    Inbox,
    Today,
    Done,
}

/// 待办实体。字段为 snake_case，与前端 Todo 类型一一对应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub status: TodoStatus,
    pub tags: Vec<String>,
    pub due_at: Option<String>,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// 新建待办入参
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTodoInput {
    pub title: String,
    pub status: Option<TodoStatus>,
    pub tags: Option<Vec<String>>,
    pub due_at: Option<String>,
    pub source: String,
}

/// 更新待办补丁：仅应用出现的字段
#[derive(Debug, Clone, Deserialize, Default)]
pub struct UpdateTodoPatch {
    pub title: Option<String>,
    pub due_at: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<TodoStatus>,
}

/// 列表过滤条件
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TodoFilter {
    pub status: Option<TodoStatus>,
    pub tag: Option<String>,
    pub limit: Option<usize>,
}

/// 数据层错误：至少包含 NotFound / Validation / Storage
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not found")]
    NotFound,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl serde::Serialize for StoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// 待办存储契约。backend Task 1 提供 SQLite 实现，前端命令层只依赖此 trait。
pub trait TodoStore: Send + Sync + 'static {
    fn create(&self, input: CreateTodoInput) -> Result<Todo, StoreError>;
    fn list(&self, filter: TodoFilter) -> Result<Vec<Todo>, StoreError>;
    fn update(&self, id: &str, patch: UpdateTodoPatch) -> Result<Todo, StoreError>;
    fn complete(&self, id: &str) -> Result<Todo, StoreError>;
    fn delete(&self, id: &str) -> Result<(), StoreError>;
}
