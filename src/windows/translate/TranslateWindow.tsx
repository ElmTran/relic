import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Copy, CornerDownLeft, X } from "lucide-react";
import { events, ipc } from "../../lib/ipc";

const SOURCE_MAX_LINES = 3;

export function TranslateWindow() {
  const [source, setSource] = useState("");
  const sourceRef = useRef<HTMLTextAreaElement>(null);

  // 原文框高度随内容变化，最多 3 行，超出后滚动
  useLayoutEffect(() => {
    const el = sourceRef.current;
    if (!el) return;
    const lineHeight = parseFloat(getComputedStyle(el).lineHeight);
    const max = lineHeight * SOURCE_MAX_LINES;
    el.style.height = "auto";
    const full = el.scrollHeight;
    el.style.height = `${Math.min(full, max)}px`;
    // 舍入误差会让单行也出现滚动条，只在真正超过上限时才允许滚动
    el.style.overflowY = full > max + 1 ? "auto" : "hidden";
  }, [source]);

  useEffect(() => {
    const unlisten = events.onShow(({ mode, text }) => {
      if (text !== null) setSource(text);
      if (mode === "input") requestAnimationFrame(() => sourceRef.current?.focus());
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        ipc.hide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="group flex h-full flex-col bg-surface">
      <header data-tauri-drag-region className="flex h-9 shrink-0 items-center pl-3.5 pr-1.5">
        <span className="pointer-events-none flex-1 text-xs text-fg-muted">中文 → English</span>
        <IconButton label="关闭" onClick={() => ipc.hide()}>
          <X size={15} />
        </IconButton>
      </header>

      <textarea
        ref={sourceRef}
        value={source}
        onChange={(e) => setSource(e.target.value)}
        placeholder="输入要翻译的文字"
        rows={1}
        spellCheck={false}
        className="mx-3.5 resize-none overflow-hidden bg-transparent outline-none placeholder:text-fg-muted"
      />

      <div className="mx-3.5 my-2 h-px shrink-0 bg-divider" />

      <div className="min-h-0 flex-1 overflow-y-auto px-3.5 select-text">
        <p className="text-fg-muted">译文会显示在这里</p>
      </div>

      <footer className="flex h-9 shrink-0 items-center justify-end gap-0.5 px-1.5 opacity-0 transition-opacity duration-150 group-hover:opacity-100">
        <TextButton icon={<Copy size={13} />}>复制</TextButton>
        <TextButton icon={<CornerDownLeft size={13} />}>回填</TextButton>
      </footer>
    </div>
  );
}

function IconButton(props: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={props.label}
      aria-label={props.label}
      onClick={props.onClick}
      className="grid size-7 place-items-center rounded-md opacity-45 transition-[opacity,background-color] duration-150 hover:bg-hover hover:opacity-100 active:bg-active"
    >
      {props.children}
    </button>
  );
}

function TextButton(props: { icon: React.ReactNode; children: React.ReactNode }) {
  return (
    <button
      type="button"
      className="flex h-7 items-center gap-1.5 rounded-md px-2 text-xs text-fg-muted transition-colors duration-150 hover:bg-hover hover:text-fg active:bg-active"
    >
      {props.icon}
      {props.children}
    </button>
  );
}
