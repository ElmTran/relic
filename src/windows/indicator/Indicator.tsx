import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { events, ipc, type IndicatorState } from "../../lib/ipc";

/** 内联翻译时光标附近的小浮标：翻译中 / 出错原因。窗口大小跟随内容 */
export function Indicator() {
  const [state, setState] = useState<IndicatorState>({ state: "working" });
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = events.onIndicator(setState);
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useLayoutEffect(() => {
    const el = contentRef.current;
    if (!el) return;
    ipc.fitIndicator(el.offsetWidth, el.offsetHeight);
  }, [state]);

  return (
    <div className="surface h-full">
      <div ref={contentRef} className="inline-flex h-8 items-center gap-2 px-3 text-xs whitespace-nowrap">
        {state.state === "working" ? (
          <>
            <span className="size-1.5 animate-pulse rounded-full bg-accent" />
            <span className="text-fg-muted">翻译中…</span>
          </>
        ) : (
          <span className="text-danger">{state.message}</span>
        )}
      </div>
    </div>
  );
}
