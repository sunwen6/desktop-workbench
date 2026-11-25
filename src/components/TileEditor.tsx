import { useState } from "react";
import type { TileInput, TileType } from "@/lib/ipc";

interface TileEditorProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (input: TileInput) => void | Promise<void>;
}

/**
 * 新增磁贴表单：首版只支持 应用(App) / 文件夹(Folder) 两种类型。
 * target 为 应用 bundle id 或 文件夹路径；只登记，不在此执行。
 */
export default function TileEditor({ open, onClose, onSubmit }: TileEditorProps) {
  const [type, setType] = useState<TileType>("Folder");
  const [label, setLabel] = useState("");
  const [target, setTarget] = useState("");

  if (!open) return null;

  const submit = async () => {
    if (!label.trim() || !target.trim()) return;
    await onSubmit({ type, label: label.trim(), target: target.trim() });
    setLabel("");
    setTarget("");
    onClose();
  };

  return (
    <div className="tile-editor" role="dialog" aria-label="添加磁贴">
      <h3>添加磁贴</h3>
      <label className="tile-field">
        类型
        <select value={type} onChange={(e) => setType(e.target.value as TileType)}>
          <option value="Folder">文件夹</option>
          <option value="App">应用</option>
        </select>
      </label>
      <label className="tile-field">
        名称
        <input value={label} onChange={(e) => setLabel(e.target.value)} placeholder="如 项目A" />
      </label>
      <label className="tile-field">
        {type === "Folder" ? "文件夹路径" : "应用 Bundle ID"}
        <input
          value={target}
          onChange={(e) => setTarget(e.target.value)}
          placeholder={type === "Folder" ? "/Users/me/项目A" : "com.apple.Terminal"}
        />
      </label>
      <div className="tile-editor-actions">
        <button onClick={onClose}>取消</button>
        <button className="primary" onClick={() => void submit()} disabled={!label.trim() || !target.trim()}>
          添加
        </button>
      </div>
    </div>
  );
}
