//! SQLite 真源：待办数据的本地持久化实现。
//! 执行流程：打开 WAL 数据库并建表 -> 入参校验 -> 事务化写入 -> 映射为 IPC 契约 Todo。

use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::model::{CreateTodoInput, StoreError, Todo, TodoFilter, TodoStore, TodoStatus, UpdateTodoPatch};

const MAX_TITLE_LENGTH: usize = 500;
const MAX_TAG_LENGTH: usize = 100;
const MAX_TAGS: usize = 50;

/// SQLite 待办仓库。连接置于 Mutex 内，保证 UI 与 MCP 的写操作串行化。
pub struct SqliteTodoStore {
    conn: Mutex<Connection>,
}

impl SqliteTodoStore {
    /// 打开（或创建）SQLite 数据库；启用 WAL 并建立表与索引。
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let conn = Connection::open(path).map_err(|e| StoreError::Storage(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| StoreError::Storage(e.to_string()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('Inbox','Today','Done')),
                tags TEXT NOT NULL DEFAULT '[]',
                due_at TEXT,
                source TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_todos_status_created_at ON todos(status, created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_todos_created_at ON todos(created_at DESC);",
        )
        .map_err(|e| StoreError::Storage(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// 内存数据库，仅供测试使用。
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::open(Path::new(":memory:"))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, StoreError> {
        self.conn.lock().map_err(|e| StoreError::Storage(format!("SQLite 连接锁损坏: {e}")))
    }
}

fn validate_title(title: &str) -> Result<String, StoreError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(StoreError::Validation("标题不能为空".into()));
    }
    if trimmed.chars().count() > MAX_TITLE_LENGTH {
        return Err(StoreError::Validation(format!("标题超过 {MAX_TITLE_LENGTH} 字符")));
    }
    Ok(trimmed.to_string())
}

fn validate_tags(tags: Option<Vec<String>>) -> Result<Vec<String>, StoreError> {
    match tags {
        None => Ok(Vec::new()),
        Some(values) => {
            if values.len() > MAX_TAGS {
                return Err(StoreError::Validation(format!("标签数量超过 {MAX_TAGS}")));
            }
            values.into_iter().map(|tag| {
                let trimmed = tag.trim();
                if trimmed.is_empty() || trimmed.chars().count() > MAX_TAG_LENGTH {
                    Err(StoreError::Validation(format!("标签必须非空且不超过 {MAX_TAG_LENGTH} 字符")))
                } else {
                    Ok(trimmed.to_string())
                }
            }).collect()
        }
    }
}

fn validate_due_at(due_at: Option<String>) -> Result<Option<String>, StoreError> {
    match due_at {
        None => Ok(None),
        Some(value) => {
            DateTime::parse_from_rfc3339(&value)
                .map(|_| Some(value))
                .map_err(|_| StoreError::Validation("due_at 必须是 ISO8601/RFC3339 时间".into()))
        }
    }
}

fn row_to_todo(row: &rusqlite::Row<'_>) -> rusqlite::Result<Todo> {
    let tags_json: String = row.get("tags")?;
    let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Todo {
        id: row.get("id")?,
        title: row.get("title")?,
        status: match row.get::<_, String>("status")?.as_str() {
            "Today" => TodoStatus::Today,
            "Done" => TodoStatus::Done,
            _ => TodoStatus::Inbox,
        },
        tags,
        due_at: row.get("due_at")?,
        source: row.get("source")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

fn get_todo(conn: &Connection, id: &str) -> Result<Todo, StoreError> {
    conn.query_row("SELECT * FROM todos WHERE id = ?1", params![id], row_to_todo)
        .optional()
        .map_err(|e| StoreError::Storage(e.to_string()))?
        .ok_or(StoreError::NotFound)
}

impl TodoStore for SqliteTodoStore {
    fn create(&self, input: CreateTodoInput) -> Result<Todo, StoreError> {
        let title = validate_title(&input.title)?;
        let tags = validate_tags(input.tags)?;
        let due_at = validate_due_at(input.due_at)?;
        let now = Utc::now().to_rfc3339();
        let status = input.status.unwrap_or(TodoStatus::Inbox);
        let todo = Todo {
            id: Uuid::new_v4().to_string(),
            title,
            status,
            tags,
            due_at,
            source: if input.source.trim().is_empty() { "mcp".into() } else { input.source.trim().into() },
            created_at: now.clone(),
            updated_at: now.clone(),
            completed_at: if status == TodoStatus::Done { Some(now) } else { None },
        };
        let tags_json = serde_json::to_string(&todo.tags).map_err(|e| StoreError::Storage(e.to_string()))?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO todos (id,title,status,tags,due_at,source,created_at,updated_at,completed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![todo.id, todo.title, format!("{:?}", todo.status), tags_json, todo.due_at, todo.source, todo.created_at, todo.updated_at, todo.completed_at],
        ).map_err(|e| StoreError::Storage(e.to_string()))?;
        Ok(todo)
    }

    fn list(&self, filter: TodoFilter) -> Result<Vec<Todo>, StoreError> {
        let mut sql = String::from("SELECT * FROM todos WHERE 1=1");
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(status) = filter.status {
            sql.push_str(" AND status = ?");
            params_vec.push(Box::new(format!("{status:?}")));
        }
        if let Some(tag) = filter.tag {
            let trimmed = tag.trim();
            if trimmed.is_empty() { return Err(StoreError::Validation("标签过滤不能为空".into())); }
            sql.push_str(" AND EXISTS (SELECT 1 FROM json_each(tags) WHERE value = ?)");
            params_vec.push(Box::new(trimmed.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC");
        if let Some(limit) = filter.limit {
            sql.push_str(" LIMIT ?");
            params_vec.push(Box::new(limit as i64));
        }
        let refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let conn = self.lock()?;
        let mut stmt = conn.prepare(&sql).map_err(|e| StoreError::Storage(e.to_string()))?;
        let rows = stmt.query_map(refs.as_slice(), row_to_todo).map_err(|e| StoreError::Storage(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| StoreError::Storage(e.to_string()))
    }

    fn update(&self, id: &str, patch: UpdateTodoPatch) -> Result<Todo, StoreError> {
        let mut conn = self.lock()?;
        let current = get_todo(&conn, id)?;
        let title = match patch.title { Some(t) => validate_title(&t)?, None => current.title };
        let tags = match patch.tags { Some(t) => validate_tags(Some(t))?, None => current.tags };
        let due_at = match patch.due_at { Some(d) => validate_due_at(Some(d))?, None => current.due_at };
        let status = patch.status.unwrap_or(current.status);
        let completed_at = if status == TodoStatus::Done {
            current.completed_at.or_else(|| Some(Utc::now().to_rfc3339()))
        } else { None };
        let tags_json = serde_json::to_string(&tags).map_err(|e| StoreError::Storage(e.to_string()))?;
        let tx = conn.transaction().map_err(|e| StoreError::Storage(e.to_string()))?;
        tx.execute(
            "UPDATE todos SET title=?1, tags=?2, due_at=?3, status=?4, updated_at=?5, completed_at=?6 WHERE id=?7",
            params![title, tags_json, due_at, format!("{status:?}"), Utc::now().to_rfc3339(), completed_at, id],
        ).map_err(|e| StoreError::Storage(e.to_string()))?;
        tx.commit().map_err(|e| StoreError::Storage(e.to_string()))?;
        get_todo(&conn, id)
    }

    fn complete(&self, id: &str) -> Result<Todo, StoreError> {
        self.update(id, UpdateTodoPatch { status: Some(TodoStatus::Done), ..Default::default() })
    }

    fn delete(&self, id: &str) -> Result<(), StoreError> {
        let conn = self.lock()?;
        let changed = conn.execute("DELETE FROM todos WHERE id=?1", params![id]).map_err(|e| StoreError::Storage(e.to_string()))?;
        if changed == 0 { Err(StoreError::NotFound) } else { Ok(()) }
    }
}
