import type { Todo } from "@/lib/ipc";

interface TodoRowProps {
  todo: Todo;
  onComplete: (id: string) => void;
  onDelete?: (id: string) => void;
  onMoveToday?: (id: string) => void;
}

/** 单条待办：复选框完成 + 标题 + 状态/标签/来源标注 */
export default function TodoRow({ todo, onComplete, onDelete, onMoveToday }: TodoRowProps) {
  const isAgent = todo.source.startsWith("agent:");

  return (
    <li className="todo-row" data-id={todo.id}>
      <input
        type="checkbox"
        className="todo-check"
        aria-label={`完成 ${todo.title}`}
        checked={todo.status === "Done"}
        onChange={() => onComplete(todo.id)}
      />
      <span className="todo-title">{todo.title}</span>
      {todo.status === "Today" && <span className="badge badge-today">今天</span>}
      {todo.tags.map((t) => (
        <span key={t} className="badge badge-tag">
          #{t}
        </span>
      ))}
      {isAgent && <span className="badge badge-source">{todo.source}</span>}
      {todo.status !== "Today" && onMoveToday && (
        <button className="todo-act" onClick={() => onMoveToday(todo.id)}>
          今天
        </button>
      )}
      {onDelete && (
        <button className="todo-act todo-del" onClick={() => onDelete(todo.id)}>
          删除
        </button>
      )}
    </li>
  );
}
