import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { TranslateEvent } from "./bindings/TranslateEvent";
import type { TranslateRequest } from "./bindings/TranslateRequest";

export type { TranslateEvent, TranslateRequest };
export type ResolvedTheme = "light" | "dark";

export interface ShowPayload {
  mode: "selection" | "input";
  /** null 表示保留窗口里现有内容 */
  text: string | null;
}

export const ipc = {
  hide: () => invoke<void>("hide"),
  getTheme: () => invoke<ResolvedTheme>("get_theme"),
  resizeToContent: (height: number) => invoke<void>("resize_to_content", { height }),
  translate: (req: TranslateRequest, onEvent: (e: TranslateEvent) => void) => {
    const channel = new Channel<TranslateEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("translate", { req, channel });
  },
  cancel: () => invoke<void>("cancel"),
};

export const events = {
  onShow: (cb: (p: ShowPayload) => void): Promise<UnlistenFn> =>
    listen<ShowPayload>("relic://show", (e) => cb(e.payload)),
  onTheme: (cb: (t: ResolvedTheme) => void): Promise<UnlistenFn> =>
    listen<ResolvedTheme>("relic://theme", (e) => cb(e.payload)),
};

/** 把解析后的主题同步到 <html data-theme>，并跟随后续变化 */
export async function syncTheme(): Promise<UnlistenFn> {
  const apply = (t: ResolvedTheme) => {
    document.documentElement.dataset.theme = t;
  };
  const unlisten = await events.onTheme(apply);
  apply(await ipc.getTheme());
  return unlisten;
}
