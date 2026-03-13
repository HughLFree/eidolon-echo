/** Settings panel entry: provider + profile configuration persisted to backend database. */

import React, { useEffect, useState } from "react";
import { getName, getVersion } from "@tauri-apps/api/app";
import { BACKEND_BASE_URL } from "./config";
import {
  createAiProvider,
  createProfile,
  listAiProviders,
  listProfiles,
  updateAiProvider,
  updateProfile
} from "./api/settings";
import "./settings.css";

const DEEPSEEK_PROVIDER_ID = "deepseek-main";
const DEFAULT_PROFILE_ID = "profile-default";
const ROLEPLAY_PROFILE_ID = "profile-roleplay";

const PAGE_OVERVIEW = "overview";
const PAGE_API = "api";
const PAGE_DEFAULT = "default";
const PAGE_ROLEPLAY = "roleplay";

const settingsPages = [
  {
    id: PAGE_OVERVIEW,
    title: "概览",
    description: "版本与结构说明"
  },
  {
    id: PAGE_API,
    title: "API 设置",
    description: "模型连接参数"
  },
  {
    id: PAGE_DEFAULT,
    title: "默认模式",
    description: "常规对话配置"
  },
  {
    id: PAGE_ROLEPLAY,
    title: "角色扮演",
    description: "角色设定与开场白"
  }
];

const defaultProviderForm = {
  apiKey: "",
  baseUrl: "https://api.deepseek.com/v1",
  modelName: "deepseek-chat",
  temperature: "0.7",
  maxTokens: ""
};

const defaultModeForm = {
  avatarPath: "/assets/pet/ava.png",
  systemPrompt:
    "你是一个桌宠 AI 助手。\n\n要求：\n- 回答简洁、明确、可执行。\n- 优先使用中文回复。\n- 不要编造事实；不确定时明确说明不确定。",
  contextLimit: "12"
};

const roleplayModeForm = {
  avatarPath: "/assets/pet/av3a.png",
  systemPrompt: "你是一个有角色设定的 AI 助手，回答要贴合角色语气。",
  openingMessage: "",
  contextLimit: "12"
};

function normalizeOptionalNumber(raw, { min = null, max = null } = {}) {
  const text = String(raw ?? "").trim();
  if (!text) {
    return null;
  }

  const value = Number(text);
  if (!Number.isFinite(value)) {
    throw new Error("数字格式不正确");
  }

  let next = value;
  if (typeof min === "number") {
    next = Math.max(min, next);
  }
  if (typeof max === "number") {
    next = Math.min(max, next);
  }
  return next;
}

function normalizeOptionalInteger(raw, { min = null, max = null } = {}) {
  const number = normalizeOptionalNumber(raw, { min, max });
  if (number === null) {
    return null;
  }
  return Math.trunc(number);
}

function trimOrNull(raw) {
  const text = String(raw ?? "").trim();
  return text ? text : null;
}

export default function SettingsApp() {
  const [activePage, setActivePage] = useState(PAGE_OVERVIEW);
  const [appName, setAppName] = useState("桌宠配置中心");
  const [appVersion, setAppVersion] = useState("0.1.0");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [statusText, setStatusText] = useState("");
  const [statusKind, setStatusKind] = useState("neutral");

  const [providerId, setProviderId] = useState(DEEPSEEK_PROVIDER_ID);
  const [providerExists, setProviderExists] = useState(false);
  const [providerForm, setProviderForm] = useState(defaultProviderForm);

  const [defaultProfileId, setDefaultProfileId] = useState(DEFAULT_PROFILE_ID);
  const [defaultProfileExists, setDefaultProfileExists] = useState(false);
  const [defaultForm, setDefaultForm] = useState(defaultModeForm);

  const [roleplayProfileId, setRoleplayProfileId] = useState(ROLEPLAY_PROFILE_ID);
  const [roleplayProfileExists, setRoleplayProfileExists] = useState(false);
  const [roleplayForm, setRoleplayForm] = useState(roleplayModeForm);

  useEffect(() => {
    let mounted = true;

    async function loadAppMeta() {
      try {
        const [name, version] = await Promise.all([getName(), getVersion()]);
        if (!mounted) {
          return;
        }
        setAppName(name || "桌宠配置中心");
        setAppVersion(version || "0.1.0");
      } catch (_error) {
        if (mounted) {
          setAppName("桌宠配置中心");
          setAppVersion("0.1.0");
        }
      }
    }

    void loadAppMeta();

    return () => {
      mounted = false;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      setLoading(true);
      setStatusText("");

      try {
        const [providers, profiles] = await Promise.all([
          listAiProviders(BACKEND_BASE_URL, { withDisabled: true }),
          listProfiles(BACKEND_BASE_URL)
        ]);

        if (cancelled) {
          return;
        }

        const provider =
          providers.find((item) => String(item.provider_type).toLowerCase().includes("deepseek")) ||
          providers.find((item) => item.is_default) ||
          providers[0] ||
          null;

        if (provider) {
          setProviderExists(true);
          setProviderId(provider.id);
          setProviderForm({
            apiKey: provider.api_key_ref || "",
            baseUrl: provider.base_url || "",
            modelName: provider.model_name || "",
            temperature:
              typeof provider.temperature === "number" ? String(provider.temperature) : "",
            maxTokens: typeof provider.max_tokens === "number" ? String(provider.max_tokens) : ""
          });
        }

        const defaultProfile = profiles.find((item) => item.mode === "default") || null;
        if (defaultProfile) {
          setDefaultProfileExists(true);
          setDefaultProfileId(defaultProfile.id);
          setDefaultForm({
            avatarPath: defaultProfile.avatar_path || "",
            systemPrompt: defaultProfile.system_prompt || "",
            contextLimit:
              typeof defaultProfile.context_limit === "number"
                ? String(defaultProfile.context_limit)
                : defaultModeForm.contextLimit
          });
        }

        const roleplayProfile = profiles.find((item) => item.mode === "roleplay") || null;
        if (roleplayProfile) {
          setRoleplayProfileExists(true);
          setRoleplayProfileId(roleplayProfile.id);
          setRoleplayForm({
            avatarPath: roleplayProfile.avatar_path || "",
            systemPrompt: roleplayProfile.system_prompt || "",
            openingMessage: roleplayProfile.opening_message || "",
            contextLimit:
              typeof roleplayProfile.context_limit === "number"
                ? String(roleplayProfile.context_limit)
                : roleplayModeForm.contextLimit
          });
        }
      } catch (error) {
        if (!cancelled) {
          setStatusKind("error");
          setStatusText(`加载失败：${error.message}`);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void bootstrap();

    return () => {
      cancelled = true;
    };
  }, []);

  async function onSave() {
    setSaving(true);
    setStatusText("");

    try {
      const providerPayload = {
        name: "DeepSeek",
        provider_type: "deepseek",
        base_url: trimOrNull(providerForm.baseUrl),
        model_name: trimOrNull(providerForm.modelName) || "deepseek-chat",
        api_key_ref: trimOrNull(providerForm.apiKey),
        enabled: true,
        is_default: true,
        temperature: normalizeOptionalNumber(providerForm.temperature, { min: 0, max: 2 }),
        max_tokens: normalizeOptionalInteger(providerForm.maxTokens, { min: 1, max: 32768 })
      };

      let nextProvider;
      if (providerExists) {
        nextProvider = await updateAiProvider(BACKEND_BASE_URL, providerId, providerPayload);
      } else {
        nextProvider = await createAiProvider(BACKEND_BASE_URL, {
          id: providerId,
          ...providerPayload
        });
        setProviderExists(true);
      }

      setProviderId(nextProvider.id);

      const defaultPayload = {
        mode: "default",
        name: "默认助手",
        avatar_path: trimOrNull(defaultForm.avatarPath),
        system_prompt: trimOrNull(defaultForm.systemPrompt) || defaultModeForm.systemPrompt,
        opening_message: null,
        context_limit: normalizeOptionalInteger(defaultForm.contextLimit, { min: 1, max: 200 }) || 12,
        provider_id: nextProvider.id,
        extra_json: JSON.stringify({ provider: "deepseek" })
      };

      if (defaultProfileExists) {
        await updateProfile(BACKEND_BASE_URL, defaultProfileId, defaultPayload);
      } else {
        const createdDefault = await createProfile(BACKEND_BASE_URL, {
          id: defaultProfileId,
          ...defaultPayload
        });
        setDefaultProfileExists(true);
        setDefaultProfileId(createdDefault.id);
      }

      const roleplayPayload = {
        mode: "roleplay",
        name: "扮演助手",
        avatar_path: trimOrNull(roleplayForm.avatarPath),
        system_prompt: trimOrNull(roleplayForm.systemPrompt) || roleplayModeForm.systemPrompt,
        opening_message: trimOrNull(roleplayForm.openingMessage),
        context_limit:
          normalizeOptionalInteger(roleplayForm.contextLimit, { min: 1, max: 200 }) || 12,
        provider_id: nextProvider.id,
        extra_json: JSON.stringify({ provider: "deepseek" })
      };

      if (roleplayProfileExists) {
        await updateProfile(BACKEND_BASE_URL, roleplayProfileId, roleplayPayload);
      } else {
        const createdRoleplay = await createProfile(BACKEND_BASE_URL, {
          id: roleplayProfileId,
          ...roleplayPayload
        });
        setRoleplayProfileExists(true);
        setRoleplayProfileId(createdRoleplay.id);
      }

      setStatusKind("success");
      setStatusText("设置已保存并写入数据库");
    } catch (error) {
      setStatusKind("error");
      setStatusText(`保存失败：${error.message}`);
    } finally {
      setSaving(false);
    }
  }

  const statusClassName = (() => {
    if (!statusText) {
      return "settings-status hidden";
    }
    if (statusKind === "success") {
      return "settings-status success";
    }
    if (statusKind === "error") {
      return "settings-status error";
    }
    return "settings-status";
  })();

  function renderEditorToolbar() {
    return (
      <section className="page-toolbar">
        <p className={statusClassName}>{statusText || "状态就绪"}</p>
        <button className="save-btn" type="button" onClick={onSave} disabled={loading || saving}>
          {saving ? "保存中..." : "保存设置"}
        </button>
      </section>
    );
  }

  function renderOverview() {
    return (
      <>
        <section className="hero-card">
          <div className="hero-copy">
            <p className="hero-eyebrow">Overview</p>
            <h2>{appName}</h2>
            <p className="hero-text">
              这是一个以 AI 对话为核心的桌面助手，采用前后端分离结构。设置中心现在按用途拆成四个页面，减少不同配置之间的混用。
            </p>
          </div>
          <div className="hero-meta">
            <div className="meta-chip">
              <span>当前版本</span>
              <strong>v{appVersion}</strong>
            </div>
            <div className="meta-chip">
              <span>前端</span>
              <strong>React + Vite</strong>
            </div>
            <div className="meta-chip">
              <span>桌面壳</span>
              <strong>Tauri + Rust</strong>
            </div>
          </div>
        </section>

        <section className="card">
          <h2>你可以在这里配置什么</h2>
          <div className="overview-grid">
            <article className="info-tile">
              <h3>API 设置</h3>
              <p>配置模型服务地址、API Key、模型名、温度和最大输出长度。</p>
            </article>
            <article className="info-tile">
              <h3>默认模式</h3>
              <p>配置普通桌宠对话的人设、头像和上下文保留数量。</p>
            </article>
            <article className="info-tile">
              <h3>角色扮演</h3>
              <p>配置角色模式的系统提示词、开场白和独立头像。</p>
            </article>
            <article className="info-tile">
              <h3>保存方式</h3>
              <p>所有设置会统一写入数据库，后续请求按数据库中的 provider 和 profile 生效。</p>
            </article>
          </div>
        </section>

        <section className="card">
          <h2>模式说明</h2>
          <div className="summary-list">
            <p>
              <strong>默认模式</strong>
              适合日常简洁对话，使用默认助手设定。
            </p>
            <p>
              <strong>角色扮演模式</strong>
              适合带角色语气的连续交流，可额外定义开场白。
            </p>
            <p>
              <strong>设置建议</strong>
              先完成 API 设置，再分别调整默认模式和角色扮演模式。
            </p>
          </div>
        </section>
      </>
    );
  }

  function renderApiSettings() {
    return (
      <>
        {renderEditorToolbar()}
        <section className="card">
          <h2>DeepSeek API</h2>
          <div className="field-grid">
            <label>
              <span>API Key</span>
              <input
                value={providerForm.apiKey}
                onChange={(event) =>
                  setProviderForm((prev) => ({ ...prev, apiKey: event.target.value }))
                }
                placeholder="sk-..."
              />
            </label>
            <label>
              <span>Base URL</span>
              <input
                value={providerForm.baseUrl}
                onChange={(event) =>
                  setProviderForm((prev) => ({ ...prev, baseUrl: event.target.value }))
                }
                placeholder="https://api.deepseek.com/v1"
              />
            </label>
            <label>
              <span>Model</span>
              <input
                value={providerForm.modelName}
                onChange={(event) =>
                  setProviderForm((prev) => ({ ...prev, modelName: event.target.value }))
                }
                placeholder="deepseek-chat"
              />
            </label>
            <label>
              <span>Temperature</span>
              <input
                value={providerForm.temperature}
                onChange={(event) =>
                  setProviderForm((prev) => ({ ...prev, temperature: event.target.value }))
                }
                placeholder="0.7"
                type="number"
                min="0"
                max="2"
                step="0.1"
              />
            </label>
            <label>
              <span>Max Tokens（可选）</span>
              <input
                value={providerForm.maxTokens}
                onChange={(event) =>
                  setProviderForm((prev) => ({ ...prev, maxTokens: event.target.value }))
                }
                placeholder="例如 4096"
                type="number"
                min="1"
                step="1"
              />
            </label>
          </div>
        </section>
      </>
    );
  }

  function renderDefaultSettings() {
    return (
      <>
        {renderEditorToolbar()}
        <section className="card">
          <h2>默认模式配置</h2>
          <div className="field-grid">
            <label>
              <span>头像路径（PNG）</span>
              <input
                value={defaultForm.avatarPath}
                onChange={(event) =>
                  setDefaultForm((prev) => ({ ...prev, avatarPath: event.target.value }))
                }
                placeholder="/assets/pet/ava.png"
              />
            </label>
            <label>
              <span>Context Limit</span>
              <input
                value={defaultForm.contextLimit}
                onChange={(event) =>
                  setDefaultForm((prev) => ({ ...prev, contextLimit: event.target.value }))
                }
                placeholder="12"
                type="number"
                min="1"
                max="200"
                step="1"
              />
            </label>
            <label className="wide">
              <span>系统提示词</span>
              <textarea
                value={defaultForm.systemPrompt}
                onChange={(event) =>
                  setDefaultForm((prev) => ({ ...prev, systemPrompt: event.target.value }))
                }
                rows={8}
              />
            </label>
          </div>
        </section>
      </>
    );
  }

  function renderRoleplaySettings() {
    return (
      <>
        {renderEditorToolbar()}
        <section className="card">
          <h2>扮演模式配置</h2>
          <div className="field-grid">
            <label>
              <span>头像路径（PNG）</span>
              <input
                value={roleplayForm.avatarPath}
                onChange={(event) =>
                  setRoleplayForm((prev) => ({ ...prev, avatarPath: event.target.value }))
                }
                placeholder="/assets/pet/av3a.png"
              />
            </label>
            <label>
              <span>Context Limit</span>
              <input
                value={roleplayForm.contextLimit}
                onChange={(event) =>
                  setRoleplayForm((prev) => ({ ...prev, contextLimit: event.target.value }))
                }
                placeholder="12"
                type="number"
                min="1"
                max="200"
                step="1"
              />
            </label>
            <label className="wide">
              <span>系统提示词</span>
              <textarea
                value={roleplayForm.systemPrompt}
                onChange={(event) =>
                  setRoleplayForm((prev) => ({ ...prev, systemPrompt: event.target.value }))
                }
                rows={8}
              />
            </label>
            <label className="wide">
              <span>开场白（可选）</span>
              <textarea
                value={roleplayForm.openingMessage}
                onChange={(event) =>
                  setRoleplayForm((prev) => ({ ...prev, openingMessage: event.target.value }))
                }
                rows={4}
              />
            </label>
          </div>
        </section>
      </>
    );
  }

  function renderActivePage() {
    if (activePage === PAGE_OVERVIEW) {
      return renderOverview();
    }
    if (activePage === PAGE_API) {
      return renderApiSettings();
    }
    if (activePage === PAGE_DEFAULT) {
      return renderDefaultSettings();
    }
    return renderRoleplaySettings();
  }

  return (
    <main className="settings-shell">
      <section className="settings-panel">
        <div className="settings-content">
          <aside className="settings-sidebar" aria-label="设置导航">
            <div className="sidebar-brand">
              <p className="settings-kicker">Settings</p>
              <h1>配置中心</h1>
            </div>
            {settingsPages.map((page) => (
              <button
                key={page.id}
                className={`nav-btn ${activePage === page.id ? "active" : ""}`}
                type="button"
                onClick={() => setActivePage(page.id)}
              >
                <span className="nav-title">{page.title}</span>
                <span className="nav-desc">{page.description}</span>
              </button>
            ))}
          </aside>

          <div className="settings-body">
            {loading ? <div className="loading-wrap">加载中...</div> : renderActivePage()}
          </div>
        </div>
      </section>
    </main>
  );
}
