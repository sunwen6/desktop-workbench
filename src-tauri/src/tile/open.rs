//! 本机打开动作：仅已登记 id，App/Folder 走 /usr/bin/open；Command 仅白名单 id。

use std::collections::HashMap;
use std::process::Command;

use super::model::{TileStore, TileType};

/// 打开已登记磁贴；未知 id 返回错误。
pub fn tile_open<S: TileStore>(store: &S, id: &str) -> Result<(), String> {
    tile_open_with_whitelist(store, id, &HashMap::new())
}

/// 带命令白名单的打开动作，便于测试与受控扩展。
pub fn tile_open_with_whitelist<S: TileStore>(
    store: &S,
    id: &str,
    command_whitelist: &HashMap<String, String>,
) -> Result<(), String> {
    let tile = store.get(id)?.ok_or_else(|| "tile not found".to_string())?;
    let target = match tile.kind {
        TileType::Command => command_whitelist
            .get(&tile.target)
            .cloned()
            .ok_or_else(|| "command tile not supported".to_string())?,
        TileType::App | TileType::Folder => tile.target,
    };
    let status = Command::new("/usr/bin/open").arg(target).spawn().map_err(|e| e.to_string())?;
    drop(status); // 不等待 GUI 应用退出
    Ok(())
}
