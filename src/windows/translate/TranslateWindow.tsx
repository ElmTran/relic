import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Copy, CornerDownLeft, X } from "lucide-react";
import { events, ipc, type TranslateEvent } from "../../lib/ipc";

const SOURCE_MAX_LINES = 3;

type Status = "idle" | "translating" | "done" | "error";

interface Direction {
  fromName: string;
  toName: string;
  swapTo: string;
}

export function TranslateWindow() {
  const [source, setSource] = useState("");
  const [result, setResult] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState("");
  const [direction, setDirection] = useState<Direction | null>(null);

  const sourceRef = useRef<HTMLTextAreaElement>(null);
  const rootRef = useRef<HTMLDivElement>(null);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  /** 只处理最新一次请求的事件 */
  const requestId = useRef(0);
  const lastRequest = useRef<{ text: string; to: string | null } | null>(null);
  const statusRef = useRef(status);
  statusRef.current = status;

  const translate = useCallback((text: string, to: string | null = null) => {
    if (!text.trim()) return;
    const id = ++requestId.current;
    lastRequest.current = { text, to };
    setResult("");
    setError("");
    setStatus("translating");
    ipc.translate({ text, to }, (e: TranslateEvent) => {
      if (id !== requestId.current) return;
      switch (e.event) {
        case "started":
          setDirection(e.data);
          break;
        case "delta":
          setResult((r) => r + e.data.text);
          break;
        case "done":
          setResult(e.data.text);
          setStatus("done");
          break;
        case "error":
          setError(e.data.message);
          setStatus("error");
          break;
      }
    });
  }, []);

  const cancel = useCallback(() => {
    requestId.current++;
    ipc.cancel();
    setStatus("done");
  }, []);

  const reset = () => {
    requestId.current++;
    setResult("");
    setError("");
    setDirection(null);
    setStatus("idle");
  };

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

  // 窗口高度跟随内容：其他部分的高度 + 译文的自然高度，上限由 Rust 限制
  useEffect(() => {
    let frame = 0;
    const measure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const root = rootRef.current;
        const scroller = scrollerRef.current;
        const content = contentRef.current;
        if (!root || !scroller || !content) return;
        const desired = root.offsetHeight - scroller.clientHeight + content.offsetHeight;
        if (Math.abs(desired - window.innerHeight) >= 1) ipc.resizeToContent(desired);
      });
    };
    const observer = new ResizeObserver(measure);
    if (contentRef.current) observer.observe(contentRef.current);
    if (sourceRef.current) observer.observe(sourceRef.current);
    measure();
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);

  useEffect(() => {
    const unlisten = events.onShow(({ mode, text }) => {
      if (text !== null) {
        setSource(text);
        reset();
        if (text.trim()) translate(text);
      }
      if (mode === "input") requestAnimationFrame(() => sourceRef.current?.focus());
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [translate]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      // 正在翻译时取消，否则隐藏窗口
      if (statusRef.current === "translating") cancel();
      else ipc.hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cancel]);

  const onSourceKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // 输入法选词时的 Enter 不算
    if (e.key === "Enter" && !e.shiftKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      translate(source);
    }
  };

  const swapDirection = () => {
    if (direction && source.trim()) translate(source, direction.swapTo);
  };

  const retry = () => {
    const last = lastRequest.current;
    if (last) translate(last.text, last.to);
  };

  return (
    <div ref={rootRef} className="group flex h-full flex-col bg-surface">
      <header data-tauri-drag-region className="flex h-9 shrink-0 items-center pl-2 pr-1.5">
        {direction ? (
          <button
            type="button"
            title="切换翻译方向"
            onClick={swapDirection}
            className="rounded-md px-1.5 py-0.5 text-xs text-fg-muted transition-colors duration-150 hover:bg-hover hover:text-fg"
          >
            {direction.fromName} → {direction.toName}
          </button>
        ) : (
          <span className="pointer-events-none px-1.5 text-xs text-fg-muted">自动检测</span>
        )}
        <div data-tauri-drag-region className="h-full flex-1" />
        <IconButton label="关闭" onClick={() => ipc.hide()}>
          <X size={15} />
        </IconButton>
      </header>

      <textarea
        ref={sourceRef}
        value={source}
        onChange={(e) => setSource(e.target.value)}
        onKeyDown={onSourceKeyDown}
        placeholder="输入要翻译的文字"
        rows={1}
        spellCheck={false}
        className="mx-3.5 shrink-0 resize-none overflow-hidden bg-transparent outline-none placeholder:text-fg-muted"
      />

      <div className="mx-3.5 my-2 h-px shrink-0 bg-divider" />

      <div ref={scrollerRef} className="min-h-0 flex-1 overflow-y-auto">
        <div ref={contentRef} className="px-3.5">
          <Result status={status} result={result} error={error} onRetry={retry} />
        </div>
      </div>

      <footer className="flex h-9 shrink-0 items-center justify-end gap-0.5 px-1.5 opacity-0 transition-opacity duration-150 group-hover:opacity-100">
        <TextButton icon={<Copy size={13} />}>复制</TextButton>
        <TextButton icon={<CornerDownLeft size={13} />}>回填</TextButton>
      </footer>
    </div>
  );
}

function Result(props: { status: Status; result: string; error: string; onRetry: () => void }) {
  if (props.status === "error") {
    return (
      <p className="text-danger">
        {props.error}
        <button type="button" onClick={props.onRetry} className="ml-2 text-fg-muted underline-offset-2 hover:text-fg hover:underline">
          重试
        </button>
      </p>
    );
  }
  if (props.result) {
    return <p className="break-words whitespace-pre-wrap select-text">{props.result}</p>;
  }
  if (props.status === "translating") return <p className="text-fg-muted">翻译中…</p>;
  return <p className="text-fg-muted">按 Enter 翻译，Shift+Enter 换行</p>;
}

function IconButton(props: { label: string; onClick: () => void; children: React.ReactNode }) {
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
