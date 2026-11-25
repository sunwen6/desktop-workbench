import { useCallback, useEffect, useState } from "react";
import {
  createTodo,
  deleteTodo,
  completeTodo,
  listTodos,
  updateTodo,
  TODO_CHANGED_EVENT,
  type Todo,
  type TodoFilter,
  type TodoStatus,
} from "@/lib/ipc";

/** 列表分组视图 */
export type Group = "inbox" | "today" | "all" | "done";

const GROUP_FILTER: Record<Group, TodoFilter> = {
  inbox: { status: "Inbox", limit: 200 },
  today: { status: "Today", limit: 200 },
  all: { limit: 200 },
  done: { status: "Done", limit: 200 },
};

/**
 * 待办数据流：首屏调用 todo_list，之后监听 todos-changed 事件重新拉取。
 * 这样 UI、MCP（agent 写入）共用同一份 SQLite，任何一侧写入后 UI 都会自动刷新。
 */
export function useTodos() {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [group, setGroup] = useState<Group>("inbox");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async (g: Group = group) => {
    try {
      const list = await listTodos(GROUP_FILTER[g]);
      setTodos(list);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [group]);

  useEffect(() => {
    setLoading(true);
    void refresh(group);
  }, [group, refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    // 监听 Rust 侧 todos-changed：UI 或 agent 任一写入都触发刷新
    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen(TODO_CHANGED_EVENT, () => {
          void refresh();
        })
      )
      .then((fn) => {
        if (cancelled) fn();
        else unlisten = fn;
      })
      .catch(() => {
        // 事件桥不可用时静默：仍可手动刷新
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [refresh]);

  const add = useCallback(
    async (title: string) => {
      await createTodo({ title, source: "ui", status: "Inbox" });
      await refresh();
    },
    [refresh]
  );

  const complete = useCallback(
    async (id: string) => {
      await completeTodo(id);
      await refresh();
    },
    [refresh]
  );

  const remove = useCallback(
    async (id: string) => {
      await deleteTodo(id);
      await refresh();
    },
    [refresh]
  );

  const setStatus = useCallback(
    async (id: string, status: TodoStatus) => {
      await updateTodo(id, { status });
      await refresh();
    },
    [refresh]
  );

  return {
    todos,
    group,
    setGroup,
    error,
    loading,
    add,
    complete,
    remove,
    setStatus,
    refresh,
  };
}
