import type { Tile } from "@/lib/ipc";

interface TileGridProps {
  tiles: Tile[];
  onOpen: (id: string) => void;
  onAdd?: () => void;
  onDelete?: (id: string) => void;
  onReorder?: (id: string, order: number) => void;
}

/**
 * 快捷磁贴区：点击打开应用/文件夹（经 tile_open，只打开已登记 id）。
 * 首版支持 App/Folder 两种类型；排序用上移/下移按钮（拖拽排序留 Phase 1）。
 */
export default function TileGrid({ tiles, onOpen, onAdd, onDelete, onReorder }: TileGridProps) {
  const sorted = [...tiles].sort((a, b) => a.order - b.order);

  return (
    <div className="tile-area">
      <div className="tile-grid">
        {sorted.map((tile, idx) => (
          <div key={tile.id} className="tile" data-id={tile.id}>
            <button className="tile-btn" onClick={() => onOpen(tile.id)}>
              <span className="tile-icon" aria-hidden="true">
                {tile.type === "App" ? "▯" : tile.type === "Folder" ? "▤" : "›_"}
              </span>
              <span className="tile-label">{tile.label}</span>
            </button>
            <div className="tile-ops">
              {onReorder && idx > 0 && (
                <button
                  className="tile-op"
                  aria-label={`上移 ${tile.label}`}
                  onClick={() => onReorder(tile.id, idx - 1)}
                >
                  ↑
                </button>
              )}
              {onReorder && idx < sorted.length - 1 && (
                <button
                  className="tile-op"
                  aria-label={`下移 ${tile.label}`}
                  onClick={() => onReorder(tile.id, idx + 1)}
                >
                  ↓
                </button>
              )}
              {onDelete && (
                <button
                  className="tile-op tile-del"
                  aria-label={`删除 ${tile.label}`}
                  onClick={() => onDelete(tile.id)}
                >
                  ×
                </button>
              )}
            </div>
          </div>
        ))}
      </div>
      {onAdd && (
        <button className="tile-add" onClick={onAdd}>
          + 添加磁贴
        </button>
      )}
    </div>
  );
}
