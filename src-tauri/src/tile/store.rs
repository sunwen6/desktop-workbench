//! SQLite 磁贴仓库：WAL + 事务化排序，数据由 UI/Tauri 命令共用。

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use super::model::{Tile, TileInput, TileStore, TileType};

/// SQLite 磁贴仓库。
pub struct SqliteTileStore { conn: Mutex<Connection> }

impl SqliteTileStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tiles (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL CHECK (type IN ('App','Folder','Command')),
                label TEXT NOT NULL,
                target TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_tiles_sort_order ON tiles(sort_order);"
        ).map_err(|e| e.to_string())?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn open_in_memory() -> Result<Self, String> { Self::open(Path::new(":memory:")) }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| e.to_string())
    }

    fn reindex(&self, conn: &mut Connection) -> Result<(), String> {
        let ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM tiles ORDER BY sort_order ASC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
        };
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (index, id) in ids.iter().enumerate() {
            tx.execute("UPDATE tiles SET sort_order=?1 WHERE id=?2", params![index as i64, id]).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }

    fn row_to_tile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tile> {
        let kind = match row.get::<_, String>("type")?.as_str() {
            "App" => TileType::App,
            "Command" => TileType::Command,
            _ => TileType::Folder,
        };
        Ok(Tile {
            id: row.get("id")?,
            kind,
            label: row.get("label")?,
            target: row.get("target")?,
            order: row.get("sort_order")?,
        })
    }
}

impl TileStore for SqliteTileStore {
    fn list(&self) -> Result<Vec<Tile>, String> {
        let conn = self.lock()?;
        let tiles = {
            let mut stmt = conn.prepare("SELECT * FROM tiles ORDER BY sort_order ASC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], Self::row_to_tile).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
        };
        Ok(tiles)
    }

    fn get(&self, id: &str) -> Result<Option<Tile>, String> {
        let conn = self.lock()?;
        conn.query_row("SELECT * FROM tiles WHERE id=?1", params![id], Self::row_to_tile)
            .optional().map_err(|e| e.to_string())
    }

    fn create(&self, input: TileInput) -> Result<Tile, String> {
        let label = input.label.trim();
        let target = input.target.trim();
        if label.is_empty() || target.is_empty() { return Err("label 和 target 必填".into()); }
        let conn = self.lock()?;
        let order = conn.query_row("SELECT COUNT(*) FROM tiles", [], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?;
        let tile = Tile { id: Uuid::new_v4().to_string(), kind: input.kind, label: label.into(), target: target.into(), order };
        conn.execute(
            "INSERT INTO tiles (id,type,label,target,sort_order) VALUES (?1,?2,?3,?4,?5)",
            params![tile.id, format!("{:?}", tile.kind), tile.label, tile.target, tile.order],
        ).map_err(|e| e.to_string())?;
        Ok(tile)
    }

    fn update(&self, id: &str, patch: TileInput) -> Result<Tile, String> {
        let label = patch.label.trim();
        let target = patch.target.trim();
        if label.is_empty() || target.is_empty() { return Err("label 和 target 必填".into()); }
        let conn = self.lock()?;
        let changed = conn.execute(
            "UPDATE tiles SET type=?1,label=?2,target=?3 WHERE id=?4",
            params![format!("{:?}", patch.kind), label, target, id],
        ).map_err(|e| e.to_string())?;
        if changed == 0 { return Err("tile not found".into()); }
        conn.query_row("SELECT * FROM tiles WHERE id=?1", params![id], Self::row_to_tile).map_err(|e| e.to_string())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let mut conn = self.lock()?;
        let changed = conn.execute("DELETE FROM tiles WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
        if changed == 0 { return Err("tile not found".into()); }
        self.reindex(&mut conn)
    }

    fn reorder(&self, id: &str, target_order: i64) -> Result<Tile, String> {
        let mut conn = self.lock()?;
        let mut ids: Vec<String> = {
            let mut stmt = conn.prepare("SELECT id FROM tiles ORDER BY sort_order ASC").map_err(|e| e.to_string())?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
        };
        let from = ids.iter().position(|x| x == id).ok_or_else(|| "tile not found".to_string())?;
        let moved = ids.remove(from);
        let target = target_order.max(0).min(ids.len() as i64) as usize;
        ids.insert(target, moved.clone());
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        for (index, tile_id) in ids.iter().enumerate() {
            tx.execute("UPDATE tiles SET sort_order=?1 WHERE id=?2", params![index as i64, tile_id]).map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        conn.query_row("SELECT * FROM tiles WHERE id=?1", params![id], Self::row_to_tile).map_err(|e| e.to_string())
    }
}
