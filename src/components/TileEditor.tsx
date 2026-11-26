import { useEffect, useMemo, useState } from "react";
import { listApps, pickFolder, type AppEntry, type TileInput, type TileType } from "@/lib/ipc";

interface TileEditorProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (input: TileInput) => void | Promise<void>;
}

const TYPE_OPTIONS: { value: TileType; label: string }[] = [
  { value: "App", label: "应用" },
  { value: "Folder", label: "文件夹" },
];

/**
 * 新增磁贴表单：App / Folder 两种类型，路径均"点选"而非手敲。
 * - App：加载本机应用列表，按名称搜索后点选，target 取其 bundle id。
 * - Folder：调用原生目录选择框，target 取选中路径。
 */
export default function TileEditor({ open, onClose, onSubmit }: TileEditorProps) {
  const [type, setType] = useState<TileType>("App");
  const [label, setLabel] = useState("");
  const [target, setTarget] = useState("");

  // App 选择器状态
  const [apps, setApps] = useState<AppEntry[]>([]);
  const [appsLoading, setAppsLoading] = useState(false);
  const [appsError, setAppsError] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  // 打开时按需加载应用列表（仅 App 类型且尚未加载）
  useEffect(() => {
    if (!open || type !== "App" || apps.length > 0 || appsLoading) return;
    let cancelled = false;
    setAppsLoading(true);
    setAppsError(null);
    listApps()
      .then((list) => {
        if (!cancelled) setApps(list);
      })
      .catch(() => {
        if (!cancelled) setAppsError("应用列表加载失败");
      })
      .finally(() => {
        if (!cancelled) setAppsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, type, apps.length, appsLoading]);

  // 过滤：按显示名或 bundle id 模糊匹配
  const filteredApps = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return apps.slice(0, 60);
    return apps
      .filter(
        (a) => a.name.toLowerCase().includes(q) || a.bundle_id.toLowerCase().includes(q),
      )
      .slice(0, 60);
  }, [apps, query]);

  const reset = () => {
    setLabel("");
    setTarget("");
    setQuery("");
  };

  const submit = async () => {
    if (!label.trim() || !target.trim()) return;
    await onSubmit({ type, label: label.trim(), target: target.trim() });
    reset();
    onClose();
  };

  const pickApp = (app: AppEntry) => {
    setTarget(app.bundle_id);
    // 名称留空时用应用名自动填充，减少输入
    if (!label.trim()) setLabel(app.name);
  };

  const browseFolder = async () => {
    const path = await pickFolder();
    if (path) {
      setTarget(path);
      // 名称留空时用目录名填充
      if (!label.trim()) setLabel(path.split("/").filter(Boolean).pop() ?? path);
    }
  };

  if (!open) return null;

  const canSubmit = label.trim() !== "" && target.trim() !== "";

  return (
    <div className="tile-editor" role="dialog" aria-label="添加磁贴">
      <div className="tile-editor-head">
        <h3>添加磁贴</h3>
        <div className="seg" role="tablist" aria-label="磁贴类型">
          {TYPE_OPTIONS.map((o) => (
            <button
              key={o.value}
              role="tab"
              aria-selected={type === o.value}
              className={`seg-btn ${type === o.value ? "active" : ""}`}
              onClick={() => {
                setType(o.value);
                setTarget("");
              }}
            >
              {o.label}
            </button>
          ))}
        </div>
      </div>

      <label className="tile-field">
        名称
        <input
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder={type === "App" ? "如 终端" : "如 项目A"}
        />
      </label>

      {type === "App" ? (
        <div className="tile-field">
          <span className="tile-field-label">选择应用</span>
          <input
            className="app-search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="搜索应用名称…"
            aria-label="搜索应用"
          />
          <div className="app-list" role="listbox" aria-label="应用列表">
            {appsLoading && <div className="app-list-msg">正在扫描本机应用…</div>}
            {appsError && <div className="app-list-msg err">{appsError}</div>}
            {!appsLoading && !appsError && filteredApps.length === 0 && (
              <div className="app-list-msg">没有匹配的应用</div>
            )}
            {filteredApps.map((app) => (
              <button
                key={app.bundle_id}
                role="option"
                aria-selected={target === app.bundle_id}
                className={`app-item ${target === app.bundle_id ? "selected" : ""}`}
                onClick={() => pickApp(app)}
                title={app.bundle_id}
              >
                <span className="app-name">{app.name}</span>
                <span className="app-bid">{app.bundle_id}</span>
              </button>
            ))}
          </div>
        </div>
      ) : (
        <div className="tile-field">
          <span className="tile-field-label">文件夹路径</span>
          <div className="folder-pick">
            <input
              className="folder-path"
              value={target}
              readOnly
              placeholder="点击右侧按钮选择文件夹"
              aria-label="文件夹路径"
            />
            <button className="btn-ghost" onClick={() => void browseFolder()}>
              浏览…
            </button>
          </div>
        </div>
      )}

      <div className="tile-editor-actions">
        <button className="btn-ghost" onClick={onClose}>
          取消
        </button>
        <button className="btn-primary" onClick={() => void submit()} disabled={!canSubmit}>
          添加
        </button>
      </div>
    </div>
  );
}
