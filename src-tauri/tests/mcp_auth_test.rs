use workbench_lib::mcp::auth::McpAuth;

#[test]
fn mcp_auth_rejects_missing_and_wrong_token() {
    let auth = McpAuth::new("secret".into());
    assert!(!auth.verify(None));
    assert!(!auth.verify(Some("Bearer wrong")));
    assert!(auth.verify(Some("Bearer secret")));
}

#[test]
fn mcp_server_only_binds_loopback() {
    assert!(workbench_lib::mcp::server::is_loopback("127.0.0.1"));
    assert!(!workbench_lib::mcp::server::is_loopback("0.0.0.0"));
}
