import { useCallback, useEffect, useState } from "react";
import { Bot, Keyboard, MessageSquareText, Settings2, Zap } from "lucide-react";
import { events, ipc, type Config, type Language } from "../../lib/ipc";
import { AiPage, GeneralPage, HotkeysPage, InlinePage, PromptsPage } from "./pages";

const PAGES = [
  { id: "general", label: "通用", icon: Settings2 },
  { id: "hotkeys", label: "快捷键", icon: Keyboard },
  { id: "inline", label: "内联翻译", icon: Zap },
  { id: "ai", label: "AI 服务", icon: Bot },
  { id: "prompts", label: "提示词", icon: MessageSquareText },
] as const;

type PageId = (typeof PAGES)[number]["id"];

/** 各页面共用：当前配置、修改并保存 */
export interface PageProps {
  config: Config;
  languages: Language[];
  /** 在副本上修改后立即保存（自动保存） */
  update: (mutate: (draft: Config) => void) => void;
}

export function Settings() {
  const [page, setPage] = useState<PageId>("general");
  const [config, setConfig] = useState<Config | null>(null);
  const [languages, setLanguages] = useState<Language[]>([]);
  const [message, setMessage] = useState<{ error: boolean; text: string } | null>(null);

  const reload = useCallback(() => {
    ipc.getConfig().then(setConfig);
  }, []);

  useEffect(() => {
    reload();
    ipc.listLanguages().then(setLanguages);
    // 托盘改主题、手动改文件后同步
    const unlisten = events.onConfigChanged(reload);
    return () => {
      unlisten.then((f) => f());
    };
  }, [reload]);

  const update = useCallback(
    (mutate: (draft: Config) => void) => {
      if (!config) return;
      const next = structuredClone(config);
      mutate(next);
      setConfig(next);
      ipc.setConfig(next).then(
        (warnings) => setMessage(warnings.length ? { error: false, text: warnings.join("；") } : null),
        (e) => setMessage({ error: true, text: `没有保存：${e}` }),
      );
    },
    [config],
  );

  const props = config && { config, languages, update };

  return (
    <div className="flex h-full select-none">
      <nav className="flex w-48 shrink-0 flex-col gap-0.5 px-2 pt-3">
        {PAGES.map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => setPage(id)}
            className={`relative flex h-9 items-center gap-3 rounded-md px-3 text-[13px] transition-colors ${
              page === id ? "bg-hover" : "hover:bg-hover"
            }`}
          >
            {page === id && <span className="absolute top-2 bottom-2 left-0 w-[3px] rounded-full bg-accent" />}
            <Icon size={16} className={page === id ? "text-accent" : "text-fg-muted"} />
            {label}
          </button>
        ))}
      </nav>

      <main className="flex min-w-0 flex-1 flex-col">
        <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-4 pb-6">
          <h1 className="mb-5 text-xl font-semibold">{PAGES.find((p) => p.id === page)?.label}</h1>
          {props && (
            <>
              {page === "general" && <GeneralPage {...props} />}
              {page === "hotkeys" && <HotkeysPage {...props} />}
              {page === "inline" && <InlinePage {...props} />}
              {page === "ai" && <AiPage {...props} />}
              {page === "prompts" && <PromptsPage {...props} />}
            </>
          )}
        </div>
        {message && (
          <div
            className={`mx-6 mb-4 rounded-md border px-3 py-2 text-xs ${
              message.error ? "border-danger/40 text-danger" : "border-card-border text-fg-muted"
            } bg-card`}
          >
            {message.text}
          </div>
        )}
      </main>
    </div>
  );
}
