import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import TodoList from "../TodoList";
import type { Todo } from "@/lib/ipc";

const mk = (o: Partial<Todo>): Todo => ({
  id: "x",
  title: "t",
  status: "Inbox",
  tags: [],
  due_at: null,
  source: "ui",
  created_at: "2026-11-25T00:00:00Z",
  updated_at: "2026-11-25T00:00:00Z",
  completed_at: null,
  ...o,
});

describe("TodoList", () => {
  it("groups todos into inbox and shows done toggle", async () => {
    const todos = [
      mk({ id: "1", title: "买牛奶", status: "Inbox" }),
      mk({
        id: "2",
        title: "周报",
        status: "Today",
        source: "agent:claude",
        created_at: "2026-11-25T00:00:01Z",
        updated_at: "2026-11-25T00:00:01Z",
      }),
    ];
    const complete = vi.fn();
    render(<TodoList todos={todos} onComplete={complete} />);
    expect(screen.getByText("买牛奶")).toBeInTheDocument();
    expect(screen.getByText("周报")).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole("checkbox")[0]);
    expect(complete).toHaveBeenCalledWith("1");
  });

  it("shows agent source badge for agent-written todos", () => {
    render(
      <TodoList todos={[mk({ id: "9", title: "从 agent 写入", source: "agent:claude" })]} onComplete={vi.fn()} />
    );
    expect(screen.getByText(/agent:claude/i)).toBeInTheDocument();
  });

  it("shows empty state text when there are no todos", () => {
    render(<TodoList todos={[]} onComplete={vi.fn()} />);
    expect(screen.getByText("还没有待办，敲一行开始")).toBeInTheDocument();
  });

  it("surfaces a read-error banner when error is passed", () => {
    render(<TodoList todos={[]} onComplete={vi.fn()} error="boom" />);
    expect(screen.getByText("本地数据读取失败")).toBeInTheDocument();
  });
});
