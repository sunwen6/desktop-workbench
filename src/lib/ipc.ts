// IPC 契约层：前端唯一与 Tauri 命令/事件打交道的地方。
// 命令名与事件名严格按实施计划：todo_create/todo_list/todo_complete/todo_delete/todo_update、todos-changed。
// 前端组件只 import 这里，便于在测试里整体 mock。
import { invoke } from "@tauri-apps/api/core";

export { invoke };

export const IPC = {
  todoCreate: "todo_create",
  todoList: "todo_list",
  todoComplete: "todo_complete",
  todoDelete: "todo_delete",
  todoUpdate: "todo_update",
  windowToggle: "window_toggle",
  windowFocusCapture: "window_focus_capture",
  windowSetAlwaysOnTop: "window_set_always_on_top",
  mcpStatus: "mcp_status",
} as const;

/** 待办状态枚举，对应 Rust 侧 TodoStatus：Inbox/Today/Done */
export type TodoStatus = "Inbox" | "Today" | "Done";

/** 与 Rust 侧 Todo 结构字段一一对应（snake_case） */
export interface Todo {
  id: string;
  title: string;
  status: TodoStatus;
  tags: string[];
  due_at: string | null;
  source: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface CreateTodoInput {
  title: string;
  status?: TodoStatus;
  tags?: string[];
  due_at?: string | null;
  source: string;
}

export interface UpdateTodoPatch {
  title?: string;
  due_at?: string | null;
  tags?: string[];
  status?: TodoStatus;
}

export interface TodoFilter {
  status?: TodoStatus;
  tag?: string;
  limit?: number;
}

/** todos-changed 事件：写操作完成后由 Rust 侧 emit，前端整表重拉，不依赖 payload 结构 */
export const TODO_CHANGED_EVENT = "todos-changed";

/** tiles-changed 事件：磁贴写操作完成后由 Rust 侧 emit（commands_tile.rs 的 TILES_CHANGED_EVENT） */
export const TILES_CHANGED_EVENT = "tiles-changed";

// ---- 磁贴（Task 4）契约 ----
export const IPC_TILE = {
  tileList: "tile_list",
  tileCreate: "tile_create",
  tileDelete: "tile_delete",
  tileReorder: "tile_reorder",
  tileOpen: "tile_open",
} as const;

/** 磁贴类型：应用 / 文件夹 / 命令（首版前端只支持 App/Folder 两种） */
export type TileType = "App" | "Folder" | "Command";

/** 与 Rust 侧 Tile 结构字段一一对应 */
export interface Tile {
  id: string;
  type: TileType;
  label: string;
  target: string; // 应用 bundle id / 文件夹路径 / 已登记命令 id
  order: number;
}

export interface TileInput {
  type: TileType;
  label: string;
  target: string;
}

export const listTiles = (): Promise<Tile[]> => invoke(IPC_TILE.tileList);
export const createTile = (input: TileInput): Promise<Tile> =>
  invoke(IPC_TILE.tileCreate, { input });
export const deleteTile = (id: string): Promise<void> =>
  invoke(IPC_TILE.tileDelete, { id });
export const reorderTile = (id: string, order: number): Promise<void> =>
  invoke(IPC_TILE.tileReorder, { id, order });
export const openTile = (id: string): Promise<void> =>
  invoke(IPC_TILE.tileOpen, { id });

export const listTodos = (filter: TodoFilter = {}): Promise<Todo[]> =>
  invoke(IPC.todoList, { filter });

export const createTodo = (input: CreateTodoInput): Promise<Todo> =>
  invoke(IPC.todoCreate, { input });

export const completeTodo = (id: string): Promise<Todo> =>
  invoke(IPC.todoComplete, { id });

export const deleteTodo = (id: string): Promise<void> =>
  invoke(IPC.todoDelete, { id });

export const updateTodo = (id: string, patch: UpdateTodoPatch): Promise<Todo> =>
  invoke(IPC.todoUpdate, { id, patch });
