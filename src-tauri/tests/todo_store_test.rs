use std::fs;
use std::path::PathBuf;

use workbench_lib::todo::store::SqliteTodoStore;
use workbench_lib::todo::{CreateTodoInput, StoreError, TodoFilter, TodoStatus, TodoStore, UpdateTodoPatch};

fn input(title: &str) -> CreateTodoInput {
    CreateTodoInput {
        title: title.into(),
        status: None,
        tags: None,
        due_at: None,
        source: "ui".into(),
    }
}

#[test]
fn create_rejects_blank_title() {
    let store = SqliteTodoStore::open_in_memory().unwrap();
    let err = store.create(input("   ")).unwrap_err();
    assert!(matches!(err, StoreError::Validation(_)));
}

#[test]
fn create_rejects_invalid_due_at() {
    let store = SqliteTodoStore::open_in_memory().unwrap();
    let mut payload = input("x");
    payload.due_at = Some("not-a-time".into());
    assert!(matches!(store.create(payload).unwrap_err(), StoreError::Validation(_)));
}

#[test]
fn create_rejects_long_title_and_oversized_tags() {
    let store = SqliteTodoStore::open_in_memory().unwrap();
    assert!(matches!(store.create(input(&"a".repeat(501))).unwrap_err(), StoreError::Validation(_)));
    let mut payload = input("x");
    payload.tags = Some(vec!["tag".into(); 51]);
    assert!(matches!(store.create(payload).unwrap_err(), StoreError::Validation(_)));
}

#[test]
fn list_filters_by_status_tag_and_limit() {
    let store = SqliteTodoStore::open_in_memory().unwrap();
    let mut a = input("a");
    a.status = Some(TodoStatus::Today);
    a.tags = Some(vec!["工作".into()]);
    store.create(a).unwrap();
    store.create(input("b")).unwrap();

    let today = store.list(TodoFilter { status: Some(TodoStatus::Today), tag: None, limit: None }).unwrap();
    assert_eq!(today.len(), 1);
    assert_eq!(today[0].title, "a");

    let tagged = store.list(TodoFilter { status: None, tag: Some("工作".into()), limit: None }).unwrap();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].title, "a");

    let limited = store.list(TodoFilter { status: None, tag: None, limit: Some(1) }).unwrap();
    assert_eq!(limited.len(), 1);
}

#[test]
fn update_is_partial_and_complete_delete_work() {
    let store = SqliteTodoStore::open_in_memory().unwrap();
    let created = store.create(input("old")).unwrap();
    let updated = store.update(&created.id, UpdateTodoPatch { title: Some("new".into()), ..Default::default() }).unwrap();
    assert_eq!(updated.title, "new");
    assert_eq!(updated.status, TodoStatus::Inbox);

    let done = store.complete(&created.id).unwrap();
    assert_eq!(done.status, TodoStatus::Done);
    assert!(done.completed_at.is_some());

    store.delete(&created.id).unwrap();
    assert!(matches!(store.delete(&created.id).unwrap_err(), StoreError::NotFound));
    assert!(store.list(TodoFilter::default()).unwrap().is_empty());
}

#[test]
fn persisted_store_uses_wal() {
    let path: PathBuf = std::env::temp_dir().join(format!("wb-todo-{}.db", uuid::Uuid::new_v4()));
    {
        let store = SqliteTodoStore::open(&path).unwrap();
        store.create(input("持久化")).unwrap();
    }
    let conn = rusqlite::Connection::open(&path).unwrap();
    let mode: String = conn.query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
    assert_eq!(mode, "wal");
    let store = SqliteTodoStore::open(&path).unwrap();
    assert_eq!(store.list(TodoFilter::default()).unwrap()[0].title, "持久化");
    fs::remove_file(path).ok();
}
