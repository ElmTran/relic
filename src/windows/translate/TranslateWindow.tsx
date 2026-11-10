import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Copy, CornerDownLeft, Pin, X } from "lucide-react";
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
  /** 底栏的短暂提示：已复制、回填失败原因 */
  const [notice, setNotice] = useState("");
  const noticeTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  /** 终端模式（双击 Alt 时前台是终端）的目标语言；null 表示普通模式 */
  const [composeTo, setComposeTo] = useState<string | null>(null);
  /** 📌 固定 = 失焦不隐藏，和设置里的"失焦时隐藏"是同一个开关 */
  const [pinned, setPinned] = useState(true);

  useEffect(() => {
    const sync = () => ipc.getConfig().then((c) => setPinned(!c.general.hide_on_blur));
    sync();
    const unlisten = events.onConfigChanged(sync);
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const togglePin = async () => {
    const config = await ipc.getConfig();
    config.general.hide_on_blur = pinned;
    setPinned(!pinned);
    ipc.setConfig(config).catch((e) => {
      setPinned(pinned);
      flash(String(e));
    });
  };

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
    const unlisten = events.onShow(({ mode, text, to }) => {
      if (text !== null) {
        // 终端模式：目标语言固定，翻译后再按 Enter 粘贴回终端
        setComposeTo(mode === "compose" ? to : null);
        setSource(text);
        reset();
        if (text.trim()) translate(text);
      }
      if (mode !== "selection") requestAnimationFrame(() => sourceRef.current?.focus());
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [translate]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        // 正在翻译时取消，否则隐藏窗口
        if (statusRef.current === "translating") cancel();
        else ipc.hide();
      } else if (e.key === "Enter" && e.ctrlKey && !e.isComposing) {
        e.preventDefault();
        actionsRef.current.pasteBack();
      } else if (e.key.toLowerCase() === "c" && e.ctrlKey && !e.shiftKey && !e.altKey && !hasTextSelection()) {
        // 窗口里没有选中文字时，Ctrl+C 复制译文
        e.preventDefault();
        actionsRef.current.copy();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cancel]);

  const flash = (text: string) => {
    clearTimeout(noticeTimer.current);
    setNotice(text);
    noticeTimer.current = setTimeout(() => setNotice(""), 2000);
  };

  const canAct = status === "done" && result.length > 0;

  const copy = () => {
    if (!canAct) return;
    ipc.copyText(result).then(
      () => flash("已复制"),
      (e) => flash(String(e)),
    );
  };

  const pasteBack = () => {
    if (!canAct) return;
    ipc.pasteBack(result).catch((e) => flash(String(e)));
  };

  // 全局快捷键处理里拿到最新的 copy / pasteBack
  const actionsRef = useRef({ copy, pasteBack });
  actionsRef.current = { copy, pasteBack };

  const onSourceKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // 输入法选词时的 Enter 不算；Ctrl+Enter 是回填，交给全局处理
    if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey && !e.nativeEvent.isComposing) {
      e.preventDefault();
      // 终端模式：原文没改过、译文已完成时，再按 Enter 粘贴回终端
      if (composeTo !== null && canAct && lastRequest.current?.text === source) pasteBack();
      else translate(source, composeTo);
    }
  };

  const swapDirection = () => {
    if (direction && source.trim()) translate(source, direction.swapTo);
  };

  const retry = () => {
    const last = lastRequest.current;
    if (last) translate(last.text, last.to);
  };

  // 终端模式下译文完成后，底栏常驻提示下一步
  const hint = notice || (composeTo !== null && canAct ? "再按 Enter 填入终端" : "");

  return (
    <div ref={rootRef} className="surface group flex h-full flex-col">
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
        <IconButton label={pinned ? "取消固定（失焦时隐藏）" : "固定（失焦不隐藏）"} active={pinned} onClick={togglePin}>
          <Pin size={14} className={`transition-transform duration-150 ${pinned ? "" : "rotate-45"}`} />
        </IconButton>
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

      <footer
        className={`flex h-9 shrink-0 items-center gap-0.5 pl-3.5 pr-1.5 transition-opacity duration-150 group-hover:opacity-100 ${
          hint ? "opacity-100" : "opacity-0"
        }`}
      >
        <span className="min-w-0 flex-1 truncate text-xs text-fg-muted">{hint}</span>
        <TextButton icon={<Copy size={13} />} disabled={!canAct} title="复制译文（Ctrl+C）" onClick={copy}>
          复制
        </TextButton>
        <TextButton icon={<CornerDownLeft size={13} />} disabled={!canAct} title="粘贴回原应用（Ctrl+Enter）" onClick={pasteBack}>
          回填
        </TextButton>
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

function IconButton(props: { label: string; active?: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      title={props.label}
      aria-label={props.label}
      aria-pressed={props.active}
      onClick={props.onClick}
      className={`grid size-7 place-items-center rounded-md transition-[opacity,background-color] duration-150 hover:bg-hover hover:opacity-100 active:bg-active ${
        props.active ? "opacity-90" : "opacity-45"
      }`}
    >
      {props.children}
    </button>
  );
}

function TextButton(props: {
  icon: React.ReactNode;
  title: string;
  disabled: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={props.title}
      disabled={props.disabled}
      onClick={props.onClick}
      className="flex h-7 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs text-fg-muted transition-colors duration-150 enabled:hover:bg-hover enabled:hover:text-fg enabled:active:bg-active disabled:opacity-40"
    >
      {props.icon}
      {props.children}
    </button>
  );
}

/** 窗口里是否选中了文字（包括原文输入框里的选区） */
function hasTextSelection(): boolean {
  const active = document.activeElement;
  if (active instanceof HTMLTextAreaElement && active.selectionStart !== active.selectionEnd) return true;
  return (window.getSelection()?.toString() ?? "").length > 0;
}
