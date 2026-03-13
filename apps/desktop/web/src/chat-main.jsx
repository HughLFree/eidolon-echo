/** Chat window entry: dedicated input box for sending messages and showing bubble reply. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./chat.css";
import { BACKEND_BASE_URL } from "./config";
import { fetchBackendHealth, sendChatMessageStream } from "./api/chat";

function ChatApp() {
  const [input, setInput] = useState("");
  const [inputDisabled, setInputDisabled] = useState(false);
  const [aiMode, setAiMode] = useState("default");
  const streamTokenRef = useRef(0);

  useEffect(() => {
    let unlistenMode;

    const bootstrap = async () => {
      try {
        const current = await invoke("get_ai_mode");
        if (typeof current?.mode === "string") {
          setAiMode(current.mode);
        }
      } catch {
        // no-op
      }

      try {
        const health = await fetchBackendHealth(BACKEND_BASE_URL);
        if (health?.ai_precheck?.ready === false) {
          const reason = (health.ai_precheck.message || "").trim();
          const precheckTip = reason || "模型连通性预检查未通过，请先在设置里确认 API 配置。";
          await pushBubble(precheckTip);
        }
      } catch {
        // no-op
      }

      unlistenMode = await listen("ai:mode-changed", async (event) => {
        const nextMode = event.payload?.mode;
        if (typeof nextMode === "string") {
          setAiMode(nextMode);
        }
      });
    };

    void bootstrap();

    return () => {
      if (unlistenMode) {
        unlistenMode();
      }
    };
  }, []);
  async function pushBubble(text) {
    try {
      await invoke("set_bubble_text", { text });
    } catch (error) {
      console.error("set_bubble_text failed:", error);
    }
  }

  async function sendMessageStream(message) {
    const token = ++streamTokenRef.current;
    const data = await sendChatMessageStream(BACKEND_BASE_URL, {
      message,
      mode: aiMode,
      onDelta: async (partialText) => {
        if (streamTokenRef.current !== token) {
          return;
        }
        await pushBubble(partialText);
      }
    });
    const responseMode = typeof data.mode === "string" ? data.mode : aiMode;
    setAiMode(responseMode);

    const backendFailure = extractBackendFailureMessage(data.reply);
    if (backendFailure) {
      throw new Error(backendFailure);
    }

    return data.reply;
  }

  async function onSubmit(event) {
    event.preventDefault();
    const text = input.trim();
    if (!text) return;

    setInput("");
    setInputDisabled(true);

    try {
      await sendMessageStream(text);
    } catch (error) {
      await pushBubble(toUserFriendlyErrorMessage(error));
    } finally {
      setInputDisabled(false);
    }
  }

  return (
    <main className="chat-shell">
      <form className="chat-form" onSubmit={onSubmit}>
        <input
          id="chat-input"
          type="text"
          placeholder="说点什么..."
          autoComplete="off"
          autoFocus
          required
          value={input}
          disabled={inputDisabled}
          onChange={(e) => setInput(e.target.value)}
        />
      </form>
    </main>
  );
}

createRoot(document.getElementById("chat-root")).render(<ChatApp />);

function toUserFriendlyErrorMessage(error) {
  const raw = String(error?.message || error || "").trim();
  const lower = raw.toLowerCase();

  if (
    lower.includes("401")
    || lower.includes("403")
    || lower.includes("unauthorized")
    || lower.includes("authentication")
    || lower.includes("invalid api key")
    || lower.includes("api key")
    || lower.includes("missing env var")
    || lower.includes("api_key is empty")
    || lower.includes("authentication fails")
    || lower.includes("invalid_request_error")
  ) {
    return "模型鉴权失败。请在“设置 -> API 设置”检查 api_key / base_url / model_name。";
  }

  if (
    lower.includes("failed to fetch")
    || lower.includes("networkerror")
    || lower.includes("connection refused")
    || lower.includes("connect")
    || lower.includes("timed out")
  ) {
    return "无法连接到后端服务，请确认桌宠已正常启动。";
  }

  return raw ? `请求失败：${raw}` : "请求失败，请稍后重试。";
}

function extractBackendFailureMessage(reply) {
  if (typeof reply !== "string") {
    return "";
  }
  const text = reply.trim();
  if (!text) {
    return "";
  }

  const marker = "请求失败：";
  const markerIndex = text.indexOf(marker);
  if (markerIndex >= 0) {
    return text.slice(markerIndex + marker.length).trim() || text;
  }

  return "";
}
