/** Settings panel entry: provider + profile configuration persisted to backend database. */

import React, { useEffect, useState } from "react";
import { getName, getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
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
const PAGE_MISC = "misc";

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
    title: "角色模式",
    description: "角色设定与开场白"
  },
  {
    id: PAGE_MISC,
    title: "其他",
    description: "数据与附加操作"
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
  avatarPath: "/assets/pet/default-avatar.png",
  systemPrompt:
    "你是一个伙伴型 AI 助手。\n\n要求：\n- 回答简洁、明确、可执行。\n- 优先使用中文回复。\n- 不要编造事实；不确定时明确说明不确定。",
  contextLimit: "12"
};

const roleplayModeForm = {
  avatarPath: "/assets/pet/roleplay-avatar.png",
  systemPrompt: "你是一个有角色设定的 AI 助手，回答要贴合角色语气。",
  openingMessage: "",
  contextLimit: "12"
};

const SETTINGS_LOAD_RETRY_MAX = 4;
const SETTINGS_LOAD_RETRY_DELAY_MS = 250;

function normalizeAvatarPath(path) {
  if (path === "/assets/pet/ava.png") {
    return "/assets/pet/default-avatar.png";
  }
  if (path === "/assets/pet/av3a.png") {
    return "/assets/pet/roleplay-avatar.png";
  }
  return path || "";
}

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
  const [appName, setAppName] = useState("Eidolon-Echo");
  const [appVersion, setAppVersion] = useState("0.1.0");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [clearingData, setClearingData] = useState(false);
  const [confirmingClear, setConfirmingClear] = useState(false);
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
        setAppName(name || "Eidolon-Echo");
        setAppVersion(version || "0.1.0");
      } catch (_error) {
        if (mounted) {
          setAppName("Eidolon-Echo");
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
    if (activePage !== PAGE_MISC && confirmingClear) {
      setConfirmingClear(false);
    }
  }, [activePage, confirmingClear]);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      setLoading(true);
      setStatusText("");

      try {
        const { providers, profiles } = await loadSettingsSnapshotWithRetry();

        if (cancelled) {
          return;
        }

        const provider =
          providers.find((item) => String(item.provider_type).toLowerCase().includes("deepseek")) ||
          providers.find((item) => item.is_default) ||
          providers[0] ||
          null;

        setProviderExists(Boolean(provider));
        setProviderId(provider?.id || DEEPSEEK_PROVIDER_ID);
        setProviderForm(
          provider
            ? {
                apiKey: provider.api_key || "",
                baseUrl: provider.base_url || "",
                modelName: provider.model_name || "",
                temperature:
                  typeof provider.temperature === "number" ? String(provider.temperature) : "",
                maxTokens:
                  typeof provider.max_tokens === "number" ? String(provider.max_tokens) : ""
              }
            : defaultProviderForm
        );

        const defaultProfile = profiles.find((item) => item.mode === "default") || null;
        setDefaultProfileExists(Boolean(defaultProfile));
        setDefaultProfileId(defaultProfile?.id || DEFAULT_PROFILE_ID);
        setDefaultForm(
          defaultProfile
            ? {
                avatarPath: normalizeAvatarPath(defaultProfile.avatar_path),
                systemPrompt: defaultProfile.system_prompt || "",
                contextLimit:
                  typeof defaultProfile.context_limit === "number"
                    ? String(defaultProfile.context_limit)
                    : defaultModeForm.contextLimit
              }
            : defaultModeForm
        );

        const roleplayProfile = profiles.find((item) => item.mode === "roleplay") || null;
        setRoleplayProfileExists(Boolean(roleplayProfile));
        setRoleplayProfileId(roleplayProfile?.id || ROLEPLAY_PROFILE_ID);
        setRoleplayForm(
          roleplayProfile
            ? {
                avatarPath: normalizeAvatarPath(roleplayProfile.avatar_path),
                systemPrompt: roleplayProfile.system_prompt || "",
                openingMessage: roleplayProfile.opening_message || "",
                contextLimit:
                  typeof roleplayProfile.context_limit === "number"
                    ? String(roleplayProfile.context_limit)
                    : roleplayModeForm.contextLimit
              }
            : roleplayModeForm
        );
      } catch (error) {
        if (!cancelled) {
          setStatusKind("error");
          setStatusText(toUserFriendlySettingsError(error, "加载失败"));
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
      await ensureBackendReady();
      const providerPayload = {
        name: "DeepSeek",
        provider_type: "deepseek",
        base_url: trimOrNull(providerForm.baseUrl),
        model_name: trimOrNull(providerForm.modelName) || "deepseek-chat",
        api_key: trimOrNull(providerForm.apiKey),
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
        avatar_path: trimOrNull(normalizeAvatarPath(defaultForm.avatarPath)),
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
        avatar_path: trimOrNull(normalizeAvatarPath(roleplayForm.avatarPath)),
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
      setStatusText(toUserFriendlySettingsError(error, "保存失败"));
    } finally {
      setSaving(false);
    }
  }

  async function onClearLocalData() {
    if (clearingData) {
      return;
    }

    if (!confirmingClear) {
      setConfirmingClear(true);
      setStatusKind("error");
      setStatusText("再次点击“确认清除”后将删除本地数据库与配置，此操作不可撤销。");
      return;
    }

    setClearingData(true);
    setStatusKind("neutral");
    setStatusText("正在清除本地数据并重建后端...");

    try {
      const result = await invoke("clear_local_data");

      setProviderExists(false);
      setProviderId(DEEPSEEK_PROVIDER_ID);
      setProviderForm(defaultProviderForm);
      setDefaultProfileExists(false);
      setDefaultProfileId(DEFAULT_PROFILE_ID);
      setDefaultForm(defaultModeForm);
      setRoleplayProfileExists(false);
      setRoleplayProfileId(ROLEPLAY_PROFILE_ID);
      setRoleplayForm(roleplayModeForm);

      const [providers, profiles] = await Promise.all([
        listAiProviders(BACKEND_BASE_URL, { withDisabled: true }),
        listProfiles(BACKEND_BASE_URL)
      ]);

      const provider =
        providers.find((item) => String(item.provider_type).toLowerCase().includes("deepseek")) ||
        providers.find((item) => item.is_default) ||
        providers[0] ||
        null;

      if (provider) {
        setProviderExists(true);
        setProviderId(provider.id);
        setProviderForm({
          apiKey: provider.api_key || "",
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
                avatarPath: normalizeAvatarPath(defaultProfile.avatar_path),
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
                avatarPath: normalizeAvatarPath(roleplayProfile.avatar_path),
                systemPrompt: roleplayProfile.system_prompt || "",
                openingMessage: roleplayProfile.opening_message || "",
                contextLimit:
            typeof roleplayProfile.context_limit === "number"
              ? String(roleplayProfile.context_limit)
              : roleplayModeForm.contextLimit
        });
      }

      setStatusKind("success");
      setStatusText(`本地数据已清除，后端已重建。数据目录：${result.dataDir || result.data_dir}`);
      setConfirmingClear(false);
    } catch (error) {
      setStatusKind("error");
      setStatusText(toUserFriendlySettingsError(error, "清除失败"));
      setConfirmingClear(false);
    } finally {
      setClearingData(false);
    }
  }

  function onCancelClear() {
    if (clearingData) {
      return;
    }
    setConfirmingClear(false);
    setStatusKind("neutral");
    setStatusText("已取消清除本地数据。");
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
              这是一个以 AI 对话为核心的桌面助手，采用前后端分离结构。
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
              <p>配置普通模式对话的人设、头像和上下文保留数量。</p>
            </article>
            <article className="info-tile">
              <h3>角色模式</h3>
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
              <strong>角色模式</strong>
              适合带角色语气的连续交流，可额外定义开场白。
            </p>
            <p>
              <strong>设置建议</strong>
              先完成 API 设置，再分别调整默认模式和角色模式。
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
                placeholder="/assets/pet/default-avatar.png"
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
          <h2>角色模式配置</h2>
          <div className="field-grid">
            <label>
              <span>头像路径（PNG）</span>
              <input
                value={roleplayForm.avatarPath}
                onChange={(event) =>
                  setRoleplayForm((prev) => ({ ...prev, avatarPath: event.target.value }))
                }
                placeholder="/assets/pet/roleplay-avatar.png"
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

  function renderMiscSettings() {
    return (
      <>
        <section className="page-toolbar">
          <p className={statusClassName}>
            {statusText || "清除会重建本地数据库。若提示失败，请先退出独立启动的后端进程。"}
          </p>
        </section>
        <section className="card danger-card">
          <h2>本地数据</h2>
          <div className="danger-copy">
            <p>
              清除后会删除本地聊天记录、provider 配置、角色配置和 SQLite 数据库，然后立即重建一个全新的本地数据目录。
            </p>
            <p>这个操作不可撤销，当前存储的 `api_key` 也会一起删除。</p>
          </div>
          <div className="danger-actions">
            <button
              className="danger-btn"
              type="button"
              onClick={onClearLocalData}
              disabled={loading || saving || clearingData}
            >
              {clearingData ? "清除中..." : confirmingClear ? "确认清除" : "清除本地数据"}
            </button>
            {confirmingClear && !clearingData ? (
              <button className="danger-cancel-btn" type="button" onClick={onCancelClear}>
                取消
              </button>
            ) : null}
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
    if (activePage === PAGE_ROLEPLAY) {
      return renderRoleplaySettings();
    }
    return renderMiscSettings();
  }

  return (
    <main className="settings-shell">
      <section className="settings-panel">
        <div className="settings-content">
          <aside className="settings-sidebar" aria-label="设置导航">
            <div className="sidebar-brand">
              <p className="settings-kicker">Settings</p>
              <h1>Eidolon-Echo</h1>
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

async function ensureBackendReady() {
  try {
    await invoke("ensure_backend_ready");
  } catch (error) {
    const message = String(error?.message || error || "").trim();
    throw new Error(message || "backend startup failed");
  }
}

async function loadSettingsSnapshot() {
  await ensureBackendReady();
  const [providers, profiles] = await Promise.all([
    listAiProviders(BACKEND_BASE_URL, { withDisabled: true }),
    listProfiles(BACKEND_BASE_URL)
  ]);
  return { providers, profiles };
}

async function loadSettingsSnapshotWithRetry() {
  let lastError = null;

  for (let i = 0; i < SETTINGS_LOAD_RETRY_MAX; i += 1) {
    try {
      return await loadSettingsSnapshot();
    } catch (error) {
      lastError = error;
      if (!isNetworkLikeError(error) || i === SETTINGS_LOAD_RETRY_MAX - 1) {
        throw error;
      }
      await sleep(SETTINGS_LOAD_RETRY_DELAY_MS);
    }
  }

  throw lastError || new Error("settings load failed");
}

function isNetworkLikeError(error) {
  const raw = String(error?.message || error || "").toLowerCase();
  return (
    raw.includes("load failed")
    || raw.includes("failed to fetch")
    || raw.includes("networkerror")
    || raw.includes("connection refused")
    || raw.includes("connect")
    || raw.includes("timed out")
  );
}

function sleep(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function toUserFriendlySettingsError(error, prefix) {
  const raw = String(error?.message || error || "").trim();
  const lower = raw.toLowerCase();

  if (
    lower.includes("backend startup failed")
    || lower.includes("unable to locate backend sidecar binary")
    || lower.includes("failed to spawn backend process")
    || lower.includes("exited during startup")
    || lower.includes("load failed")
    || lower.includes("failed to fetch")
    || lower.includes("networkerror")
    || lower.includes("connection refused")
    || lower.includes("connect")
    || lower.includes("timed out")
  ) {
    const detail = raw ? ` 详情：${raw}` : "";
    return `${prefix}：无法连接到后端服务。请先完全退出应用后重试。${detail}`;
  }

  if (
    lower.includes("401")
    || lower.includes("403")
    || lower.includes("unauthorized")
    || lower.includes("authentication")
    || lower.includes("invalid api key")
    || lower.includes("api key")
    || lower.includes("authentication fails")
    || lower.includes("invalid_request_error")
  ) {
    return `${prefix}：模型鉴权失败，请检查 API 设置。`;
  }

  return raw ? `${prefix}：${raw}` : `${prefix}：请稍后重试。`;
}
