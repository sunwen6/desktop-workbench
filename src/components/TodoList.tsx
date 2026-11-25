import TodoRow from "./TodoRow";
import type { Todo } from "@/lib/ipc";

interface TodoListProps {
  todos: Todo[];
  onComplete: (id: string) => void;
  onDelete?: (id: string) => void;
  onMoveToday?: (id: string) => void;
  /** 本地数据读取失败时的错误信息；有值则显示错误横幅 */
  error?: string | null;
  loading?: boolean;
}

/**
 * 待办列表：按传入顺序渲染（服务端已按 created_at DESC 排序），勾选完成，标注来源。
 * 分组筛选（收件箱/今天/全部/已完成）由顶栏切换 filter 控制，本组件不做二次排序，
 * 以保证「新记一条 → 出现在列表顶部」的预期，且勾选目标与用户所见顺序一致。
 */
export default function TodoList({
  todos,
  onComplete,
  onDelete,
  onMoveToday,
  error,
  loading,
}: TodoListProps) {
  if (error) {
    return (
      <div className="todo-error" role="alert">
        <span>本地数据读取失败</span>
        {error ? <span>：{error}</span> : null}
      </div>
    );
  }
  if (loading) {
    return <div className="todo-empty">加载中…</div>;
  }
  if (todos.length === 0) {
    return <div className="todo-empty">还没有待办，敲一行开始</div>;
  }

  return (
    <ul className="todo-list" aria-label="待办列表">
      {todos.map((t) => (
        <TodoRow
          key={t.id}
          todo={t}
          onComplete={onComplete}
          onDelete={onDelete}
          onMoveToday={onMoveToday}
        />
      ))}
    </ul>
  );
}
