//! MCP 本地 HTTP 服务与工具/资源执行层。
//! 约束：仅绑定 127.0.0.1；POST/GET /mcp 均要求 Bearer token；
//! 工具只读写 TodoStore，不暴露文件/命令/网络能力。

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

use crate::todo::{StoreError, TodoStore, TodoStatus};
use super::auth::McpAuth;

/// 5 个 MCP 工具名。
pub fn tool_names() -> [&'static str; 5] {
    ["add_todo", "list_todos", "complete_todo", "delete_todo", "update_todo"]
}

/// 仅允许回环地址绑定。
pub fn is_loopback(host: &str) -> bool { host == "127.0.0.1" }

/// MCP 首选端口与候选区间（17321..=17330）。
pub const MCP_PORT_START: u16 = 17321;
pub const MCP_PORT_END: u16 = 17330;

/// 依次尝试候选端口并完成绑定，绑定失败的端口自动跳到下一个，避免“先探测后绑定”的竞态。
/// 对“端口刚被释放仍在 TIME_WAIT”这类瞬时占用，做最多 3 轮重试（间隔 100ms）。
/// `port == 0` 时按区间 17321..=17330 逐个尝试；其余值只尝试该端口。
fn bind_server(bind: &str, port: u16) -> Result<(Server, u16), String> {
    let candidates: Vec<u16> = if port == 0 {
        (MCP_PORT_START..=MCP_PORT_END).collect()
    } else {
        vec![port]
    };
    let mut last_error = String::new();
    for attempt in 0..3 {
        for candidate in &candidates {
            match Server::http(format!("{bind}:{candidate}")) {
                Ok(server) => {
                    let actual = server.server_addr().to_ip()
                        .ok_or_else(|| "MCP server address is not IP".to_string())?.port();
                    return Ok((server, actual));
                }
                Err(e) => last_error = e.to_string(),
            }
        }
        if attempt < 2 {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    if port == 0 {
        Err(format!("MCP 端口 {MCP_PORT_START}-{MCP_PORT_END} 全部被占用: {last_error}"))
    } else {
        Err(format!("MCP 端口 {port} 绑定失败: {last_error}"))
    }
}

fn status_from(value: Option<&Value>) -> Result<Option<TodoStatus>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => match s.as_str() {
            "Inbox" => Ok(Some(TodoStatus::Inbox)),
            "Today" => Ok(Some(TodoStatus::Today)),
            "Done" => Ok(Some(TodoStatus::Done)),
            _ => Err("invalid status".into()),
        },
        _ => Err("invalid status".into()),
    }
}

fn err_string(err: StoreError) -> String { err.to_string() }

/// 执行单个 MCP 工具，返回 JSON-RPC result。
pub fn execute_tool<S: TodoStore + ?Sized>(store: &S, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "add_todo" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let tags = match args.get("tags") {
                Some(Value::Array(items)) => Some(items.iter().filter_map(|v| v.as_str().map(|x| x.to_string())).collect()),
                _ => None,
            };
            let due_at = args.get("due_at").and_then(|v| v.as_str()).map(|x| x.to_string());
            let source = args.get("source").and_then(|v| v.as_str()).unwrap_or("mcp").to_string();
            store.create(crate::todo::CreateTodoInput {
                title, status: status_from(args.get("status"))?, tags, due_at, source,
            }).map(|t| serde_json::to_value(t).unwrap()).map_err(err_string)
        }
        "list_todos" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize).or(Some(50));
            let tag = args.get("tag").and_then(|v| v.as_str()).map(|x| x.to_string());
            store.list(crate::todo::TodoFilter { status: status_from(args.get("status"))?, tag, limit })
                .map(|items| serde_json::to_value(items).unwrap()).map_err(err_string)
        }
        "complete_todo" => {
            let id = args.get("id").and_then(|v| v.as_str()).ok_or("id is required")?;
            store.complete(id).map(|t| serde_json::to_value(t).unwrap()).map_err(err_string)
        }
        "delete_todo" => {
            let id = args.get("id").and_then(|v| v.as_str()).ok_or("id is required")?;
            store.delete(id).map(|_| json!({"deleted": true})).map_err(err_string)
        }
        "update_todo" => {
            let id = args.get("id").and_then(|v| v.as_str()).ok_or("id is required")?;
            let title = args.get("title").and_then(|v| v.as_str()).map(|x| x.to_string());
            let tags = match args.get("tags") {
                Some(Value::Array(items)) => Some(items.iter().filter_map(|v| v.as_str().map(|x| x.to_string())).collect()),
                _ => None,
            };
            let due_at = args.get("due_at").and_then(|v| v.as_str()).map(|x| x.to_string());
            let patch = crate::todo::UpdateTodoPatch { title, due_at, tags, status: status_from(args.get("status"))? };
            store.update(id, patch).map(|t| serde_json::to_value(t).unwrap()).map_err(err_string)
        }
        _ => Err(format!("unknown tool: {name}")),
    }
}

/// 读取只读资源；仅支持 todos://today 与 todos://inbox。
pub fn read_resource<S: TodoStore + ?Sized>(store: &S, uri: &str) -> Result<String, String> {
    let status = match uri {
        "todos://today" => TodoStatus::Today,
        "todos://inbox" => TodoStatus::Inbox,
        _ => return Err(format!("unknown resource: {uri}")),
    };
    let todos = store.list(crate::todo::TodoFilter { status: Some(status), tag: None, limit: Some(50) }).map_err(err_string)?;
    serde_json::to_string(&todos).map_err(|e| e.to_string())
}

/// MCP 运行状态。
#[derive(Debug, Clone, serde::Serialize)]
pub struct McpStatus { pub running: bool, pub port: u16 }

/// 启动本地 HTTP(SSE) MCP 服务；返回状态与关闭函数。
pub fn start_mcp_server<S: TodoStore + 'static + ?Sized>(
    store: Arc<S>,
    auth: McpAuth,
    bind: &str,
    port: u16,
) -> Result<(McpStatus, Box<dyn FnOnce() + Send>), String> {
    let noop: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    start_mcp_server_with_emitter(store, auth, bind, port, noop)
}

/// 启动带写操作事件通知的 MCP 服务。
/// 所有写工具（add/complete/delete/update）成功后调用 `on_changed`，用于 Tauri emit `todos-changed`。
pub fn start_mcp_server_with_emitter<S: TodoStore + 'static + ?Sized>(
    store: Arc<S>,
    auth: McpAuth,
    bind: &str,
    port: u16,
    on_changed: Arc<dyn Fn() + Send + Sync>,
) -> Result<(McpStatus, Box<dyn FnOnce() + Send>), String> {
    if !is_loopback(bind) { return Err("MCP server must bind to 127.0.0.1".into()); }
    let (server, actual_port) = bind_server(bind, port)?;
    let running = Arc::new(Mutex::new(true));
    let running_for_thread = running.clone();
    let handle = std::thread::spawn(move || {
        for request in server.incoming_requests() {
            let authorized = auth.verify(request.headers().iter()
                .find(|h| h.field.as_str() == "Authorization")
                .map(|h| h.value.as_str()));
            let path = request.url().to_string();
            let method = request.method().as_str().to_string();
            if path == "/health" && method == "GET" {
                let _ = request.respond(Response::from_string("{\"status\":\"ok\"}").with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()));
                continue;
            }
            if !authorized {
                let _ = request.respond(Response::from_string("{\"error\":\"unauthorized\"}").with_status_code(401));
                continue;
            }
            if path == "/mcp" && method == "GET" {
                let _ = request.respond(Response::from_string("event: ready\ndata: {\"running\":true}\n\n").with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"text/event-stream"[..]).unwrap()));
                continue;
            }
            if path != "/mcp" || method != "POST" {
                let _ = request.respond(Response::from_string("{\"error\":\"not found\"}").with_status_code(404));
                continue;
            }
            let mut body = String::new();
            let mut req = request;
            let _ = req.as_reader().read_to_string(&mut body);
            let response = match serde_json::from_str::<Value>(&body) {
                Ok(payload) => {
                    let id = payload.get("id").cloned().unwrap_or(Value::Null);
                    let result = match payload.get("method").and_then(|v| v.as_str()) {
                        Some("tools/list") => Ok(json!({"tools": tool_names().iter().map(|n| json!({"name": n})).collect::<Vec<_>>()})),
                        Some("tools/call") => {
                            let name = payload.get("params").and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                            let arguments = payload.get("params").and_then(|p| p.get("arguments")).unwrap_or(&Value::Null);
                            let result = execute_tool(&*store, name, arguments);
                            if result.is_ok() && matches!(name, "add_todo" | "complete_todo" | "delete_todo" | "update_todo") {
                                on_changed();
                            }
                            result
                        }
                        Some("resources/list") => Ok(json!({"resources": [{"uri":"todos://today"},{"uri":"todos://inbox"}]})),
                        Some("resources/read") => {
                            let uri = payload.get("params").and_then(|p| p.get("uri")).and_then(|v| v.as_str()).unwrap_or("");
                            read_resource(&*store, uri).map(|text| json!({"contents":[{"uri":uri,"mimeType":"application/json","text":text}]}))
                        }
                        Some(other) => Err(format!("unknown method: {other}")),
                        None => Err("missing method".into()),
                    };
                    match result {
                        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
                        Err(message) => json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32000,"message":message}}),
                    }
                }
                Err(e) => json!({"jsonrpc":"2.0","id":Value::Null,"error":{"code":-32700,"message":e.to_string()}}),
            };
            let _ = req.respond(Response::from_string(response.to_string()).with_header(
                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap()));
        }
        *running_for_thread.lock().unwrap() = false;
    });
    let close = Box::new(move || drop(handle));
    Ok((McpStatus { running: true, port: actual_port }, close))
}
