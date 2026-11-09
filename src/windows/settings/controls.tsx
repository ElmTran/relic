import { useEffect, useRef, useState } from "react";
import { Eye, EyeOff, X } from "lucide-react";
import { ipc } from "../../lib/ipc";

/** 一组设置：标题 + 卡片 */
export function Section(props: { title?: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <section className="mb-6">
      {(props.title || props.action) && (
        <div className="mb-2 flex items-center justify-between px-1">
          <h2 className="text-[13px] font-semibold">{props.title}</h2>
          {props.action}
        </div>
      )}
      <div className="divide-y divide-divider rounded-lg border border-card-border bg-card">{props.children}</div>
    </section>
  );
}

/** 一行设置：左边说明，右边控件 */
export function Row(props: { label: string; desc?: React.ReactNode; children: React.ReactNode; stacked?: boolean }) {
  return (
    <div className={props.stacked ? "space-y-2 px-4 py-3" : "flex min-h-14 items-center gap-6 px-4 py-2.5"}>
      <div className="min-w-0 flex-1">
        <div className="text-[13px]">{props.label}</div>
        {props.desc && <div className="mt-0.5 text-xs text-fg-muted">{props.desc}</div>}
      </div>
      <div className={props.stacked ? "" : "flex shrink-0 items-center"}>{props.children}</div>
    </div>
  );
}

export function Toggle(props: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={props.checked}
      disabled={props.disabled}
      onClick={() => props.onChange(!props.checked)}
      className={`relative h-5 w-10 rounded-full border transition-colors duration-150 disabled:opacity-40 ${
        props.checked ? "border-accent bg-accent" : "border-fg-muted bg-transparent"
      }`}
    >
      <span
        className={`absolute top-1/2 size-3 -translate-y-1/2 rounded-full transition-all duration-150 ${
          props.checked ? "left-[22px] bg-white dark:bg-black" : "left-[3px] bg-fg-muted"
        }`}
      />
    </button>
  );
}

const fieldClass =
  "h-8 rounded-md border border-field-border bg-field px-2.5 text-[13px] outline-none transition-colors focus:border-accent disabled:opacity-50";

export function Select<T extends string>(props: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  width?: string;
}) {
  return (
    <select
      value={props.value}
      onChange={(e) => props.onChange(e.target.value as T)}
      className={`${fieldClass} ${props.width ?? "w-44"} pr-1`}
    >
      {props.options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

/** 文本框：编辑时只改本地草稿，失焦或按 Enter 才提交（避免每个字都写一次配置） */
export function TextField(props: {
  value: string;
  onCommit: (v: string) => void;
  placeholder?: string;
  secret?: boolean;
  mono?: boolean;
  disabled?: boolean;
  width?: string;
}) {
  const [draft, setDraft] = useState(props.value);
  const [reveal, setReveal] = useState(false);
  const focused = useRef(false);

  // 外部改了值（如文件热重载）且没在编辑时，同步过来
  useEffect(() => {
    if (!focused.current) setDraft(props.value);
  }, [props.value]);

  const commit = () => {
    if (draft !== props.value) props.onCommit(draft);
  };

  return (
    <div className={`relative ${props.width ?? "w-72"}`}>
      <input
        type={props.secret && !reveal ? "password" : "text"}
        value={draft}
        placeholder={props.placeholder}
        disabled={props.disabled}
        spellCheck={false}
        onChange={(e) => setDraft(e.target.value)}
        onFocus={() => (focused.current = true)}
        onBlur={() => {
          focused.current = false;
          commit();
        }}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
        className={`${fieldClass} w-full placeholder:text-fg-muted ${props.mono ? "font-mono text-xs" : ""} ${
          props.secret ? "pr-8" : ""
        }`}
      />
      {props.secret && (
        <button
          type="button"
          title={reveal ? "隐藏" : "显示"}
          onClick={() => setReveal(!reveal)}
          className="absolute top-1/2 right-1.5 grid size-6 -translate-y-1/2 place-items-center rounded text-fg-muted hover:bg-hover hover:text-fg"
        >
          {reveal ? <EyeOff size={14} /> : <Eye size={14} />}
        </button>
      )}
    </div>
  );
}

/** 多行文本：失焦时提交。validate 返回错误信息时不提交，并显示在下方 */
export function TextArea(props: {
  value: string;
  onCommit: (v: string) => void;
  rows?: number;
  mono?: boolean;
  placeholder?: string;
  validate?: (v: string) => string | null;
}) {
  const [draft, setDraft] = useState(props.value);
  const [error, setError] = useState<string | null>(null);
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) setDraft(props.value);
  }, [props.value]);

  return (
    <div>
      <textarea
        value={draft}
        rows={props.rows ?? 4}
        placeholder={props.placeholder}
        spellCheck={false}
        onChange={(e) => setDraft(e.target.value)}
        onFocus={() => (focused.current = true)}
        onBlur={() => {
          focused.current = false;
          const err = props.validate?.(draft) ?? null;
          setError(err);
          if (!err && draft !== props.value) props.onCommit(draft);
        }}
        className={`w-full resize-y rounded-md border border-field-border bg-field px-2.5 py-2 text-[13px] leading-relaxed outline-none transition-colors placeholder:text-fg-muted focus:border-accent ${
          props.mono ? "font-mono text-xs" : ""
        }`}
      />
      {error && <div className="mt-1 text-xs text-danger">{error}</div>}
    </div>
  );
}

export function Button(props: {
  onClick: () => void;
  children: React.ReactNode;
  title?: string;
  disabled?: boolean;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      title={props.title}
      disabled={props.disabled}
      onClick={props.onClick}
      className={`flex h-7 items-center gap-1.5 rounded-md px-2.5 text-xs transition-colors enabled:hover:bg-hover enabled:active:bg-active disabled:opacity-40 ${
        props.danger ? "text-danger" : "text-fg-muted enabled:hover:text-fg"
      }`}
    >
      {props.children}
    </button>
  );
}

// ---- 热键录入 ----

const MODIFIERS: ["ctrlKey" | "altKey" | "shiftKey" | "metaKey", string][] = [
  ["ctrlKey", "Ctrl"],
  ["altKey", "Alt"],
  ["shiftKey", "Shift"],
  ["metaKey", "Super"],
];

/** 把 KeyboardEvent.code 转成全局热键能识别的键名；修饰键本身返回 null */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(code)) return null;
  return code; // Space、Backquote、ArrowUp 等，直接用 code 名
}

/** 显示用：Super 显示成 Win */
function display(hotkey: string): string[] {
  return hotkey ? hotkey.split("+").map((k) => (k === "Super" ? "Win" : k)) : [];
}

/**
 * 点击后录入：按下组合键即保存，Esc 取消，Backspace 清空（关闭这个热键）。
 * 录入期间暂停全局热键，否则按下要录的组合键会直接触发翻译。
 */
export function HotkeyField(props: { value: string; onCommit: (v: string) => void }) {
  const [recording, setRecording] = useState(false);
  const [live, setLive] = useState<string[]>([]);
  const [hint, setHint] = useState("");
  const ref = useRef<HTMLButtonElement>(null);

  const start = () => {
    setRecording(true);
    setLive([]);
    setHint("");
    ipc.suspendHotkeys(true);
  };

  const stop = async (value?: string) => {
    setRecording(false);
    setLive([]);
    // 先恢复原来的热键，再保存新值（保存时会按新配置重新注册）
    await ipc.suspendHotkeys(false);
    if (value !== undefined && value !== props.value) props.onCommit(value);
    ref.current?.blur();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    const mods = MODIFIERS.filter(([k]) => e[k]).map(([, name]) => name);
    const key = keyName(e.code);
    if (!key) {
      setLive(mods); // 只按了修饰键，先显示出来
      return;
    }
    if (mods.length === 0) {
      if (e.code === "Escape") return void stop();
      if (e.code === "Backspace" || e.code === "Delete") return void stop("");
      if (!/^F\d{1,2}$/.test(key)) {
        setHint("需要配合 Ctrl / Alt / Shift / Win");
        return;
      }
    }
    stop([...mods, key].join("+"));
  };

  const keys = recording ? live : display(props.value);
  return (
    <div className="flex flex-col items-end gap-1">
      <div className="flex items-center gap-1">
        <button
          ref={ref}
          type="button"
          onClick={() => (recording ? stop() : start())}
          onKeyDown={onKeyDown}
          onBlur={() => recording && stop()}
          className={`flex h-8 min-w-36 items-center justify-center gap-1 rounded-md border px-2 text-xs transition-colors ${
            recording ? "border-accent bg-field" : "border-field-border bg-field hover:border-fg-muted"
          }`}
        >
          {keys.length > 0 ? (
            keys.map((k) => (
              <kbd key={k} className="rounded border border-field-border bg-card px-1.5 py-0.5 font-sans text-[11px]">
                {k}
              </kbd>
            ))
          ) : (
            <span className="text-fg-muted">{recording ? "按下快捷键…" : "未设置"}</span>
          )}
        </button>
        {props.value && !recording && (
          <button
            type="button"
            title="清除（关闭这个热键）"
            onClick={() => props.onCommit("")}
            className="grid size-7 place-items-center rounded-md text-fg-muted hover:bg-hover hover:text-fg"
          >
            <X size={14} />
          </button>
        )}
      </div>
      {recording && <div className="text-[11px] text-fg-muted">{hint || "Esc 取消，Backspace 清除"}</div>}
    </div>
  );
}
