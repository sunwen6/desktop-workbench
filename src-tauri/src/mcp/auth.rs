//! MCP Bearer token 校验：严格匹配 `Bearer <token>`，不读取或写入仓库内 token。

/// MCP 鉴权器。
#[derive(Clone)]
pub struct McpAuth { token: String }

impl McpAuth {
    pub fn new(token: String) -> Self { Self { token } }

    /// 校验 Authorization 头；仅接受与本地 token 完全一致的 Bearer 值。
    pub fn verify(&self, header: Option<&str>) -> bool {
        header.map(|h| h == format!("Bearer {}", self.token).as_str()).unwrap_or(false)
    }
}
