import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, it, expect, vi } from "vitest";
import TileGrid from "../TileGrid";
import type { Tile } from "@/lib/ipc";

describe("TileGrid", () => {
  it("opens a tile by id", async () => {
    const open = vi.fn();
    const tiles: Tile[] = [
      { id: "t1", type: "Folder", label: "项目A", target: "/tmp", order: 0 },
    ];
    render(<TileGrid tiles={tiles} onOpen={open} />);
    await userEvent.click(screen.getByRole("button", { name: "项目A" }));
    expect(open).toHaveBeenCalledWith("t1");
  });

  it("renders app and folder tiles with accessible names", () => {
    const tiles: Tile[] = [
      { id: "a", type: "App", label: "终端", target: "com.apple.Terminal", order: 0 },
      { id: "b", type: "Folder", label: "下载", target: "~/Downloads", order: 1 },
    ];
    render(<TileGrid tiles={tiles} onOpen={vi.fn()} />);
    expect(screen.getByRole("button", { name: "终端" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "下载" })).toBeInTheDocument();
  });

  it("shows an add-tile entry that opens the editor", async () => {
    const onAdd = vi.fn();
    render(<TileGrid tiles={[]} onOpen={vi.fn()} onAdd={onAdd} />);
    await userEvent.click(screen.getByRole("button", { name: /添加磁贴/ }));
    expect(onAdd).toHaveBeenCalledTimes(1);
  });
});
