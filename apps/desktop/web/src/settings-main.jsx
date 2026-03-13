/** Settings panel entry: provider + profile configuration persisted to backend database. */

import React, { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
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

const TAB_API = "api";
const TAB_ROLEPLAY = "roleplay";

const defaultProviderForm = {
  apiKey: "",
  baseUrl: "https://api.deepseek.com/v1",
  modelName: "deepseek-chat",
  temperature: "0.7",
  maxTokens: ""
};

const defaultModeForm = {
  avatarPath: "/assets/pet/ava.png",
  systemPrompt: "你是一个简洁、可靠的桌面助手。",
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

function SettingsApp() {
  const [activeTab, setActiveTab] = useState(TAB_API);
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

  const statusClassName = useMemo(() => {
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
  }, [statusKind, statusText]);

  return (
    <main className="settings-shell">
      <section className="settings-panel">
        <header className="settings-header">
          <div>
            <p className="settings-kicker">Settings</p>
            <h1>桌宠配置中心</h1>
          </div>
        </header>

        <div className="tab-row">
          <button
            className={`tab-btn ${activeTab === TAB_API ? "active" : ""}`}
            type="button"
            onClick={() => setActiveTab(TAB_API)}
          >
            API 设置
          </button>
          <button
            className={`tab-btn ${activeTab === TAB_ROLEPLAY ? "active" : ""}`}
            type="button"
            onClick={() => setActiveTab(TAB_ROLEPLAY)}
          >
            扮演角色设置
          </button>
        </div>

        {loading ? (
          <div className="loading-wrap">加载中...</div>
        ) : (
          <div className="settings-body">
            {activeTab === TAB_API ? (
              <>
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
                      <span>Max Tokens (可选)</span>
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
                    <label className="wide">
                      <span>系统提示词</span>
                      <textarea
                        value={defaultForm.systemPrompt}
                        onChange={(event) =>
                          setDefaultForm((prev) => ({ ...prev, systemPrompt: event.target.value }))
                        }
                        rows={6}
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
                  </div>
                </section>
              </>
            ) : (
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
                  <label className="wide">
                    <span>系统提示词</span>
                    <textarea
                      value={roleplayForm.systemPrompt}
                      onChange={(event) =>
                        setRoleplayForm((prev) => ({ ...prev, systemPrompt: event.target.value }))
                      }
                      rows={7}
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
                </div>
              </section>
            )}
          </div>
        )}

        <footer className="settings-footer">
          <p className={statusClassName}>{statusText || "状态就绪"}</p>
          <button className="save-btn" type="button" onClick={onSave} disabled={loading || saving}>
            {saving ? "保存中..." : "应用设置"}
          </button>
        </footer>
      </section>
    </main>
  );
}

createRoot(document.getElementById("settings-root")).render(<SettingsApp />);
