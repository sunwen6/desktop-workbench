import { useCallback, useEffect, useState } from "react";
import QuickCapture from "./components/QuickCapture";
import TodoList from "./components/TodoList";
import TileGrid from "./components/TileGrid";
import TileEditor from "./components/TileEditor";
import { useTodos, type Group } from "./hooks/useTodos";
import { invoke, IPC, IPC_TILE, createTile, deleteTile, openTile, reorderTile, TILES_CHANGED_EVENT, type Tile, type TileInput } from "./lib/ipc";

const GROUPS: { key: Group; label: string }[] = [
  { key: "inbox", label: "收件箱" },
  { key: "today", label: "今天" },
  { key: "all", label: "全部" },
  { key: "done", label: "已完成" },
];

interface McpStatus {
  running: boolean;
  port: number;
}

export default function App() {
  const { todos, group, setGroup, error, loading, add, complete, remove, setStatus } = useTodos();
  const [tiles, setTiles] = useState<Tile[]>([]);
  const [mcp, setMcp] = useState<McpStatus>({ running: false, port: 0 });
  const [editorOpen, setEditorOpen] = useState(false);

  const loadTiles = useCallback(async () => {
    try {
      setTiles(await invoke(IPC_TILE.tileList));
    } catch {
      // 磁贴读取失败不阻塞待办主路径，留空即可
    }
  }, []);

  const loadMcp = useCallback(async () => {
    try {
      setMcp(await invoke<McpStatus>(IPC.mcpStatus));
    } catch {
      setMcp({ running: false, port: 0 });
    }
  }, []);

  useEffect(() => {
    void loadTiles();
    void loadMcp();
  }, [loadTiles, loadMcp]);

  // 监听 Rust 侧 tiles-changed：任一磁贴写操作后整表重拉，保证 UI 不滞留旧排序/旧内容
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    import("@tauri-apps/api/event")
      .then(({ listen }) => listen(TILES_CHANGED_EVENT, () => void loadTiles()))
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // 事件桥不可用时静默：磁贴仍可在下次挂载时拉取
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadTiles]);

  const handleAddTile = async (input: TileInput) => {
    await createTile(input);
    await loadTiles();
  };

  const handleDeleteTile = async (id: string) => {
    await deleteTile(id);
    await loadTiles();
  };

  const handleReorderTile = async (id: string, order: number) => {
    await reorderTile(id, order);
    await loadTiles();
  };

  return (
    <div className="app">
      <header className="topbar">
        <span className="app-title">待办工作台</span>
        <nav className="groups" aria-label="分组筛选">
          {GROUPS.map((g) => (
            <button
              key={g.key}
              className={`group-btn ${group === g.key ? "active" : ""}`}
              aria-pressed={group === g.key}
              onClick={() => setGroup(g.key)}
            >
              {g.label}
            </button>
          ))}
        </nav>
        <span className={`mcp-dot ${mcp.running ? "on" : "off"}`} title={mcp.running ? `MCP 运行中 :${mcp.port}` : "MCP 未运行"}>
          {mcp.running ? "● MCP 运行中" : "○ MCP 未运行"}
        </span>
      </header>

      <main className="content">
        <section className="left">
          <QuickCapture onCreate={add} />
          <TodoList
            todos={todos}
            onComplete={complete}
            onDelete={remove}
            onMoveToday={(id) => void setStatus(id, "Today")}
            error={error}
            loading={loading}
          />
        </section>

        <aside className="right">
          <h2 className="right-title">快捷磁贴</h2>
          <TileGrid
            tiles={tiles}
            onOpen={(id) => void openTile(id)}
            onAdd={() => setEditorOpen(true)}
            onDelete={(id) => void handleDeleteTile(id)}
            onReorder={(id, order) => void handleReorderTile(id, order)}
          />
          <TileEditor open={editorOpen} onClose={() => setEditorOpen(false)} onSubmit={handleAddTile} />
          <p className="status-line">状态：本地已保存</p>
        </aside>
      </main>
    </div>
  );
}
