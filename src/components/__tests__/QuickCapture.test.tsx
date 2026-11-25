import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import QuickCapture, { CAPTURE_PLACEHOLDER } from "../QuickCapture";

describe("QuickCapture", () => {
  it("does not create a todo for blank quick capture", async () => {
    const create = vi.fn();
    render(<QuickCapture onCreate={create} />);
    const input = screen.getByPlaceholderText(CAPTURE_PLACEHOLDER);
    await userEvent.type(input, "   {enter}");
    expect(create).not.toHaveBeenCalled();
    // 输入框保留，便于用户继续输入
    expect(input).toHaveValue("   ");
  });

  it("creates a todo for a trimmed non-empty title and clears input", async () => {
    const create = vi.fn().mockResolvedValue(undefined);
    render(<QuickCapture onCreate={create} />);
    const input = screen.getByPlaceholderText(CAPTURE_PLACEHOLDER);
    await userEvent.type(input, "回复张三邮件{enter}");
    expect(create).toHaveBeenCalledTimes(1);
    expect(create).toHaveBeenCalledWith("回复张三邮件");
    expect(input).toHaveValue("");
  });

  it("ignores whitespace-only title produced by trimming around real text", async () => {
    const create = vi.fn().mockResolvedValue(undefined);
    render(<QuickCapture onCreate={create} />);
    const input = screen.getByPlaceholderText(CAPTURE_PLACEHOLDER);
    await userEvent.type(input, "  周报  {enter}");
    expect(create).toHaveBeenCalledWith("周报");
  });
});
