//! MCP 运行时接线测试：端口选择、状态托管、端口文件、HTTP 行为与事件通知。

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use workbench_lib::commands_tile::{
    load_or_create_token, mcp_status_value, read_mcp_port_file, write_mcp_port_file, McpState,
};
use workbench_lib::mcp::auth::McpAuth;
use workbench_lib::mcp::server::{start_mcp_server_with_emitter, MCP_PORT_END, MCP_PORT_START};
use workbench_lib::mcp::McpStatus;
use workbench_lib::todo::store::SqliteTodoStore;
use workbench_lib::todo::TodoStore;

/// 真实绑定端口的用例共用这把锁串行执行，避免并行时互相占用 17321-17330。
static PORT_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn mcp_port_range_is_17321_to_17330() {
    assert_eq!(MCP_PORT_START, 17321);
    assert_eq!(MCP_PORT_END, 17330);
}

#[test]
fn mcp_state_reports_real_status() {
    let state = McpState::new();
    let empty = mcp_status_value(&state).unwrap();
    assert!(!empty.running);
    assert_eq!(empty.port, 0);

    state.set(McpStatus { running: true, port: 17321 }).unwrap();
    let actual = mcp_status_value(&state).unwrap();
    assert!(actual.running);
    assert_eq!(actual.port, 17321);
}

#[test]
fn mcp_port_file_roundtrip() {
    let dir = std::env::temp_dir().join(format!("wb-port-{}", uuid::Uuid::new_v4()));
    let path = dir.join("mcp-port");
    write_mcp_port_file(&path, 17325).unwrap();
    assert_eq!(read_mcp_port_file(&path).unwrap(), 17325);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn token_file_is_generated_with_600_permissions() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("wb-token-{}", uuid::Uuid::new_v4()));
        let path = dir.join("mcp-token");
        let token = load_or_create_token(&path).unwrap();
        assert_eq!(token, load_or_create_token(&path).unwrap());
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(dir).ok();
    }
}

fn http_request(port: u16, method: &str, path: &str, auth: Option<&str>, body: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let payload = body.unwrap_or("");
    let auth_header = auth.map(|a| format!("Authorization: {a}\r\n")).unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{auth_header}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    stream.write_all(req.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status: u16 = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

#[test]
fn mcp_falls_back_to_next_port_when_preferred_is_occupied() {
    let _guard = PORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 构造“首选端口 + 其它备用端口都被占住”的场景：先探测一个空闲端口作为期望落点，再占住区间内其余端口，
    // 这样服务只剩这一个端口可绑，消除探测与绑定之间的竞态。
    let expected = match (MCP_PORT_START..=MCP_PORT_END)
        .find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
    {
        Some(port) => port,
        None => return, // 区间内无空闲端口，无法验证，跳过
    };
    let mut blockers: Vec<std::net::TcpListener> = Vec::new();
    for port in MCP_PORT_START..=MCP_PORT_END {
        if port == expected { continue; }
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => blockers.push(listener),
            Err(_) => return, // 无法构造全占用场景，跳过
        }
    }
    let store = Arc::new(SqliteTodoStore::open_in_memory().unwrap());
    let noop: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let (status, closer) = start_mcp_server_with_emitter(
        store,
        McpAuth::new("t".into()),
        "127.0.0.1",
        0,
        noop,
    )
    .unwrap();
    assert_eq!(status.port, expected);
    let (health, _) = http_request(status.port, "GET", "/health", None, None);
    assert_eq!(health, 200);
    closer();
    drop(blockers);
}

#[test]
fn mcp_all_ports_occupied_returns_error() {
    let _guard = PORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // 区间内全部端口都被占住时，应返回明确错误而非 panic
    let mut blockers: Vec<std::net::TcpListener> = Vec::new();
    for port in MCP_PORT_START..=MCP_PORT_END {
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(listener) => blockers.push(listener),
            // 某个候选端口已被本机其它进程占用时无法构造“全占用”场景，跳过此用例
            Err(_) => return,
        }
    }
    let store = Arc::new(SqliteTodoStore::open_in_memory().unwrap());
    let noop: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let result = start_mcp_server_with_emitter(store, McpAuth::new("t".into()), "127.0.0.1", 0, noop);
    let message = match result {
        Ok(_) => panic!("全部端口占用时应返回错误"),
        Err(message) => message,
    };
    assert!(message.contains("全部被占用"), "错误信息: {message}");
    drop(blockers);
}

#[test]
fn mcp_http_health_auth_and_change_event() {
    let _guard = PORT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let store = Arc::new(SqliteTodoStore::open_in_memory().unwrap());
    let emitted = Arc::new(AtomicUsize::new(0));
    let emitted_for_server = emitted.clone();
    let emitter: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        emitted_for_server.fetch_add(1, Ordering::SeqCst);
    });
    let (status, _closer) = start_mcp_server_with_emitter(
        store.clone(),
        McpAuth::new("secret".into()),
        "127.0.0.1",
        0,
        emitter,
    )
    .unwrap();
    let port = status.port;
    assert!(status.running);
    assert!(port > 0);

    // 健康检查无需 token
    let (health_status, _) = http_request(port, "GET", "/health", None, None);
    assert_eq!(health_status, 200);

    // 无 token 调用 tools/call 返回 401
    let (no_token_status, _) = http_request(
        port,
        "POST",
        "/mcp",
        None,
        Some(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"add_todo","arguments":{"title":"x"}}}"#),
    );
    assert_eq!(no_token_status, 401);
    assert_eq!(emitted.load(Ordering::SeqCst), 0);

    // 正确 token 写入待办并触发 todos-changed 事件
    let (ok_status, response_body) = http_request(
        port,
        "POST",
        "/mcp",
        Some("Bearer secret"),
        Some(r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"add_todo","arguments":{"title":"agent 写入","source":"agent:claude"}}}"#),
    );
    assert_eq!(ok_status, 200);
    assert!(response_body.contains("agent 写入"), "响应: {response_body}");
    assert!(response_body.contains("agent:claude"), "响应: {response_body}");
    assert_eq!(emitted.load(Ordering::SeqCst), 1);

    let todos = store.list(Default::default()).unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].source, "agent:claude");
    assert_eq!(todos[0].title, "agent 写入");
}
