//! 磁贴数据模型：与前端 `src/lib/ipc.ts` 的 Tile 契约一致。

use serde::{Deserialize, Serialize};

/// 磁贴类型：App/Folder 用 /usr/bin/open 打开，Command 仅允许白名单 id。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TileType { App, Folder, Command }

/// 磁贴实体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tile {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: TileType,
    pub label: String,
    pub target: String,
    pub order: i64,
}

/// 新建/更新磁贴入参。
#[derive(Debug, Clone, Deserialize)]
pub struct TileInput {
    #[serde(alias = "type")]
    pub kind: TileType,
    pub label: String,
    pub target: String,
}

/// 磁贴仓库契约。
pub trait TileStore {
    fn list(&self) -> Result<Vec<Tile>, String>;
    fn get(&self, id: &str) -> Result<Option<Tile>, String>;
    fn create(&self, input: TileInput) -> Result<Tile, String>;
    fn update(&self, id: &str, patch: TileInput) -> Result<Tile, String>;
    fn delete(&self, id: &str) -> Result<(), String>;
    fn reorder(&self, id: &str, target_order: i64) -> Result<Tile, String>;
}
