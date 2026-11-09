import { useEffect, useState } from "react";
import { Check, FileCog, Plus, Trash2 } from "lucide-react";
import { ipc } from "../../lib/ipc";
import type { ApiFormat } from "../../lib/bindings/ApiFormat";
import type { Provider } from "../../lib/bindings/Provider";
import type { ThemeSetting } from "../../lib/bindings/ThemeSetting";
import { Button, HotkeyField, Row, Section, Select, TextArea, TextField, Toggle } from "./controls";
import type { PageProps } from "./Settings";

// ---- 通用 ----

export function GeneralPage({ config, languages, update }: PageProps) {
  const [autostart, setAutostart] = useState(false);
  const [autostartError, setAutostartError] = useState("");

  useEffect(() => {
    ipc.getAutostart().then(setAutostart);
  }, []);

  const toggleAutostart = (v: boolean) => {
    ipc.setAutostart(v).then(
      () => {
        setAutostart(v);
        setAutostartError("");
      },
      (e) => setAutostartError(String(e)),
    );
  };

  const langOptions = languages.map((l) => ({ value: l.code, label: l.name }));
  const g = config.general;

  return (
    <>
      <Section>
        <Row label="主题">
          <Select<ThemeSetting>
            value={g.theme}
            options={[
              { value: "system", label: "跟随系统" },
              { value: "light", label: "浅色" },
              { value: "dark", label: "深色" },
            ]}
            onChange={(v) => update((c) => (c.general.theme = v))}
          />
        </Row>
        <Row label="开机自启" desc={autostartError || undefined}>
          <Toggle checked={autostart} onChange={toggleAutostart} />
        </Row>
      </Section>

      <Section title="翻译方向">
        <Row label="母语" desc="原文是母语时译成第二语言，否则译成母语">
          <Select value={g.primary_lang} options={langOptions} onChange={(v) => update((c) => (c.general.primary_lang = v))} />
        </Row>
        <Row label="第二语言">
          <Select
            value={g.secondary_lang}
            options={langOptions}
            onChange={(v) => update((c) => (c.general.secondary_lang = v))}
          />
        </Row>
      </Section>

      <Section title="翻译窗口">
        <Row label="失焦时隐藏" desc="切到别的窗口时自动隐藏翻译窗口">
          <Toggle checked={g.hide_on_blur} onChange={(v) => update((c) => (c.general.hide_on_blur = v))} />
        </Row>
        <Row label="回填后保留译文" desc="回填后译文留在剪贴板里，而不是恢复成原来的内容">
          <Toggle
            checked={g.keep_result_in_clipboard}
            onChange={(v) => update((c) => (c.general.keep_result_in_clipboard = v))}
          />
        </Row>
      </Section>

      <Button onClick={() => ipc.openConfigFile()} title="直接编辑 config.toml，保存后自动生效">
        <FileCog size={14} />
        在编辑器中打开配置文件
      </Button>
    </>
  );
}

// ---- 快捷键 ----

export function HotkeysPage({ config, update }: PageProps) {
  return (
    <Section>
      <Row label="划词翻译" desc="翻译其他应用里选中的文字">
        <HotkeyField value={config.hotkeys.selection} onCommit={(v) => update((c) => (c.hotkeys.selection = v))} />
      </Row>
      <Row label="输入翻译" desc="打开翻译窗口，输入文字翻译">
        <HotkeyField value={config.hotkeys.input} onCommit={(v) => update((c) => (c.hotkeys.input = v))} />
      </Row>
    </Section>
  );
}

// ---- 内联翻译 ----

export function InlinePage({ config, languages, update }: PageProps) {
  const i = config.inline;
  return (
    <>
      <Section>
        <Row
          label="快速按两下 Alt"
          desc="在输入框里把内容就地替换成译文：有选中翻译选中部分，否则翻译整个输入框。终端里会弹出输入框"
        >
          <Toggle checked={i.enabled} onChange={(v) => update((c) => (c.inline.enabled = v))} />
        </Row>
        <Row label="目标语言">
          <Select
            value={i.target_lang}
            options={languages.map((l) => ({ value: l.code, label: l.name }))}
            onChange={(v) => update((c) => (c.inline.target_lang = v))}
          />
        </Row>
        <Row label="两次按键的最大间隔" desc="毫秒，100 到 1000">
          <TextField
            value={String(i.interval_ms)}
            width="w-24"
            onCommit={(v) => {
              const ms = Number.parseInt(v, 10);
              if (Number.isFinite(ms)) update((c) => (c.inline.interval_ms = ms));
            }}
          />
        </Row>
      </Section>

      <Section title="不启用的应用">
        <Row label="进程名" desc="每行一个，如 Code.exe" stacked>
          <TextArea
            value={i.blacklist.join("\n")}
            rows={3}
            mono
            placeholder="Code.exe"
            onCommit={(v) =>
              update(
                (c) =>
                  (c.inline.blacklist = v
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean)),
              )
            }
          />
        </Row>
      </Section>
    </>
  );
}

// ---- AI 服务 ----

const FORMATS: { value: ApiFormat; label: string; baseUrl: string; model: string }[] = [
  { value: "openai", label: "OpenAI 兼容", baseUrl: "https://api.deepseek.com", model: "deepseek-flash" },
  { value: "anthropic", label: "Anthropic", baseUrl: "https://api.anthropic.com/v1", model: "claude-haiku-5-5" },
  { value: "gemini", label: "Gemini", baseUrl: "https://generativelanguage.googleapis.com/v1beta", model: "gemini-flash-latest" },
];

export function AiPage({ config, update }: PageProps) {
  const providers = config.ai.providers;

  const add = () =>
    update((c) => {
      let n = c.ai.providers.length + 1;
      while (c.ai.providers.some((p) => p.id === `服务 ${n}`)) n++;
      c.ai.providers.push({ id: `服务 ${n}`, format: "openai", base_url: "", api_key: "", model: "", stream: true });
      if (c.ai.providers.length === 1) c.ai.active = c.ai.providers[0].id;
    });

  return (
    <>
      {providers.map((p, index) => (
        <ProviderCard
          key={index}
          provider={p}
          active={config.ai.active === p.id}
          canDelete={providers.length > 1}
          onActivate={() => update((c) => (c.ai.active = p.id))}
          onDelete={() =>
            update((c) => {
              c.ai.providers.splice(index, 1);
              if (c.ai.active === p.id) c.ai.active = c.ai.providers[0]?.id ?? "";
            })
          }
          onChange={(mutate) =>
            update((c) => {
              const target = c.ai.providers[index];
              const oldId = target.id;
              mutate(target);
              // 改名时同步"当前使用"
              if (c.ai.active === oldId) c.ai.active = target.id;
            })
          }
        />
      ))}
      <Button onClick={add}>
        <Plus size={14} />
        添加服务
      </Button>
    </>
  );
}

function ProviderCard(props: {
  provider: Provider;
  active: boolean;
  canDelete: boolean;
  onActivate: () => void;
  onDelete: () => void;
  onChange: (mutate: (p: Provider) => void) => void;
}) {
  const { provider: p, onChange } = props;
  const format = FORMATS.find((f) => f.value === p.format) ?? FORMATS[0];
  const isAnthropic = p.format === "anthropic";

  return (
    <Section
      title={p.id}
      action={
        <div className="flex items-center gap-1">
          {props.active ? (
            <span className="flex items-center gap-1 px-2 text-xs text-accent">
              <Check size={14} />
              使用中
            </span>
          ) : (
            <Button onClick={props.onActivate}>设为当前</Button>
          )}
          <Button onClick={props.onDelete} disabled={!props.canDelete} title="删除" danger>
            <Trash2 size={14} />
          </Button>
        </div>
      }
    >
      <Row label="名称">
        <TextField value={p.id} onCommit={(v) => onChange((x) => (x.id = v.trim()))} />
      </Row>
      <Row label="接口格式">
        <Select<ApiFormat>
          value={p.format}
          width="w-72"
          options={FORMATS.map((f) => ({ value: f.value, label: f.label }))}
          onChange={(v) => onChange((x) => (x.format = v))}
        />
      </Row>
      <Row label="地址" desc="会在后面拼上接口路径；多数 OpenAI 兼容服务要以 /v1 结尾">
        <TextField value={p.base_url} placeholder={format.baseUrl} mono onCommit={(v) => onChange((x) => (x.base_url = v.trim()))} />
      </Row>
      <Row label="API Key">
        <TextField value={p.api_key} secret mono onCommit={(v) => onChange((x) => (x.api_key = v.trim()))} />
      </Row>
      <Row label="模型">
        <TextField value={p.model} placeholder={format.model} mono onCommit={(v) => onChange((x) => (x.model = v.trim()))} />
      </Row>
      <Row
        label="Temperature"
        desc={isAnthropic ? "Anthropic 格式不发送这个参数" : "留空表示不发送，使用服务的默认值"}
      >
        <TextField
          value={p.temperature == null ? "" : String(p.temperature)}
          width="w-24"
          disabled={isAnthropic}
          onCommit={(v) => {
            const t = v.trim() === "" ? null : Number(v);
            if (t === null || Number.isFinite(t)) onChange((x) => (x.temperature = t));
          }}
        />
      </Row>
      <Row label="流式输出" desc="译文边生成边显示">
        <Toggle checked={p.stream} onChange={(v) => onChange((x) => (x.stream = v))} />
      </Row>
      <Row label="额外参数" desc="JSON，深度合并进请求体，如关闭思考模式" stacked>
        <TextArea
          value={p.extra_body == null ? "" : JSON.stringify(p.extra_body, null, 2)}
          rows={3}
          mono
          placeholder='{ "thinking": { "type": "disabled" } }'
          validate={(v) => {
            if (!v.trim()) return null;
            try {
              const parsed = JSON.parse(v);
              return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? null : "必须是 JSON 对象";
            } catch {
              return "不是有效的 JSON";
            }
          }}
          onCommit={(v) => onChange((x) => (x.extra_body = v.trim() ? JSON.parse(v) : null))}
        />
      </Row>
    </Section>
  );
}

// ---- 提示词 ----

export function PromptsPage({ config, update }: PageProps) {
  return (
    <>
      <p className="mb-4 px-1 text-xs text-fg-muted">
        这里是系统提示词，可用变量 {"{from}"}（原文语言）和 {"{to}"}（目标语言）。原文会作为用户消息单独发送。
      </p>
      <Section>
        <Row label="翻译" desc="翻译窗口、划词翻译使用" stacked>
          <TextArea value={config.prompts.translate} rows={7} onCommit={(v) => update((c) => (c.prompts.translate = v))} />
        </Row>
        <Row label="内联翻译" desc="快速按两下 Alt 时使用" stacked>
          <TextArea value={config.prompts.inline} rows={5} onCommit={(v) => update((c) => (c.prompts.inline = v))} />
        </Row>
      </Section>
    </>
  );
}
