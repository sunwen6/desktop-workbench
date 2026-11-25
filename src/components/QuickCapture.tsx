import { useState } from "react";

interface QuickCaptureProps {
  onCreate: (title: string) => void | Promise<void>;
}

export const CAPTURE_PLACEHOLDER = "敲一行，回车入箱…";

/**
 * 快速记录框：Enter 提交，trim 后非空才调用 onCreate。
 * 首版只传 title；`#标签`、`!今天` 解析留给 Phase 1。
 */
export default function QuickCapture({ onCreate }: QuickCaptureProps) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    const title = value.trim();
    if (!title || busy) return; // 空标题/纯空白不入库
    setBusy(true);
    try {
      await onCreate(title);
      setValue("");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="capture">
      <input
        className="capture-input"
        type="text"
        placeholder={CAPTURE_PLACEHOLDER}
        aria-label={CAPTURE_PLACEHOLDER}
        value={value}
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            void submit();
          }
        }}
      />
    </div>
  );
}
