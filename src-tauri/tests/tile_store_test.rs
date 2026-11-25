use workbench_lib::tile::{tile_open, TileInput, TileStore, TileType};

fn store() -> workbench_lib::tile::SqliteTileStore {
    workbench_lib::tile::SqliteTileStore::open_in_memory().unwrap()
}

#[test]
fn tile_crud_and_reorder() {
    let store = store();
    let a = store.create(TileInput {
        kind: TileType::Folder,
        label: "项目A".into(),
        target: "/tmp".into(),
    }).unwrap();
    let b = store.create(TileInput {
        kind: TileType::App,
        label: "终端".into(),
        target: "com.apple.Terminal".into(),
    }).unwrap();
    store.reorder(&b.id, 0).unwrap();
    let tiles = store.list().unwrap();
    assert_eq!(tiles[0].id, b.id);
    store.delete(&a.id).unwrap();
    assert_eq!(store.list().unwrap().len(), 1);
}

#[test]
fn tile_open_rejects_unknown_id() {
    let store = store();
    let err = tile_open(&store, "does-not-exist").unwrap_err();
    assert!(err.contains("not found"));
}

#[test]
fn tile_open_rejects_unregistered_command_tile() {
    let store = store();
    let tile = store.create(TileInput {
        kind: TileType::Command,
        label: "命令".into(),
        target: "anything".into(),
    }).unwrap();
    let err = tile_open(&store, &tile.id).unwrap_err();
    assert!(err.contains("not supported"));
}
