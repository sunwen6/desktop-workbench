use serde_json::json;
use workbench_lib::mcp::server::{execute_tool, read_resource, tool_names};
use workbench_lib::todo::store::SqliteTodoStore;

fn store() -> SqliteTodoStore { SqliteTodoStore::open_in_memory().unwrap() }

#[test]
fn mcp_add_todo_creates_todo() {
    let store = store();
    let result = execute_tool(&store, "add_todo", &json!({"title":"从 agent 写入","source":"agent:claude"})).unwrap();
    assert!(result["id"].as_str().is_some());
    assert_eq!(result["source"], "agent:claude");
    assert!(result["created_at"].as_str().is_some());
}

#[test]
fn mcp_rejects_blank_title_and_invalid_due_at() {
    let store = store();
    assert!(execute_tool(&store, "add_todo", &json!({"title":"   "})).is_err());
    assert!(execute_tool(&store, "add_todo", &json!({"title":"x","due_at":"not-a-time"})).is_err());
}

#[test]
fn mcp_tools_and_resources_work() {
    let store = store();
    let created = execute_tool(&store, "add_todo", &json!({"title":"x"})).unwrap();
    let id = created["id"].as_str().unwrap();
    let listed = execute_tool(&store, "list_todos", &json!({"status":"Inbox"})).unwrap();
    assert!(listed.as_array().unwrap().iter().any(|t| t["id"] == id));
    let done = execute_tool(&store, "complete_todo", &json!({"id":id})).unwrap();
    assert_eq!(done["status"], "Done");
    let inbox = read_resource(&store, "todos://inbox").unwrap();
    assert!(!inbox.contains(id));
    assert_eq!(execute_tool(&store, "delete_todo", &json!({"id":id})).unwrap(), json!({"deleted":true}));
    let updated = execute_tool(&store, "add_todo", &json!({"title":"old","tags":["a"]})).unwrap();
    let uid = updated["id"].as_str().unwrap();
    let patched = execute_tool(&store, "update_todo", &json!({"id":uid,"title":"new"})).unwrap();
    assert_eq!(patched["title"], "new");
}

#[test]
fn mcp_tools_and_resources_are_complete() {
    let names = tool_names();
    for expected in ["add_todo","list_todos","complete_todo","delete_todo","update_todo"] {
        assert!(names.contains(&expected));
    }
    let store = store();
    assert!(read_resource(&store, "todos://today").is_ok());
    assert!(read_resource(&store, "todos://inbox").is_ok());
    assert!(read_resource(&store, "todos://other").is_err());
}
