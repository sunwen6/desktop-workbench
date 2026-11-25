//! Tile / MCP Tauri 命令符号契约：前端 ipc.ts 中的命令名必须在 Rust 侧存在。
#[allow(unused_imports)]
use workbench_lib::commands_tile::{
    mcp_status, tile_create, tile_delete, tile_list, tile_open, tile_reorder,
};

#[test]
fn tile_and_mcp_commands_are_defined() {
    let _ = tile_list;
    let _ = tile_create;
    let _ = tile_delete;
    let _ = tile_reorder;
    let _ = tile_open;
    let _ = mcp_status;
}
