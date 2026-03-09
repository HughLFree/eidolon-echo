/** Chat window entry: dedicated input box for sending messages and showing bubble reply. */

import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./chat.css";
import { BACKEND_BASE_URL } from "./config";
import { readActiveSessionId, writeActiveSessionId } from "./session";
import { sendChatMessage } from "./api/chat";

function ChatApp() {
  const [sessionId, setSessionId] = useState(null);
  const [input, setInput] = useState("");
  const [inputDisabled, setInputDisabled] = useState(false);
  const [aiMode, setAiMode] = useState("default");

  useEffect(() => {
    let unlistenMode;

    const bootstrap = async () => {
      try {
        const current = await invoke("get_ai_mode");
        if (typeof current?.mode === "string") {
          setAiMode(current.mode);
          let activeSid = null;
          try {
            activeSid = await invoke("get_active_session_id", { mode: current.mode });
          } catch {
            // no-op
          }
          const fallbackSid = readActiveSessionId(current.mode);
          const nextSid = typeof activeSid === "number" ? activeSid : fallbackSid;
          setSessionId(nextSid);
        }
      } catch {
        // no-op
      }

      unlistenMode = await listen("ai:mode-changed", async (event) => {
        const nextMode = event.payload?.mode;
        if (typeof nextMode === "string") {
          setAiMode(nextMode);
          let activeSid = null;
          try {
            activeSid = await invoke("get_active_session_id", { mode: nextMode });
          } catch {
            // no-op
          }
          const fallbackSid = readActiveSessionId(nextMode);
          const nextSid = typeof activeSid === "number" ? activeSid : fallbackSid;
          setSessionId(nextSid);
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

  useEffect(() => {
    const onKeyDown = (event) => {
      if (event.key === "Escape") {
        void invoke("hide_pet");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  async function pushBubble(text) {
    try {
      await invoke("set_bubble_text", { text });
    } catch (error) {
      console.error("set_bubble_text failed:", error);
    }
  }

  async function sendMessage(message) {
    let tauriSessionId = null;
    try {
      tauriSessionId = await invoke("get_active_session_id", { mode: aiMode });
    } catch {
      // no-op
    }
    const effectiveSessionId =
      (typeof tauriSessionId === "number" ? tauriSessionId : null) ??
      readActiveSessionId(aiMode) ??
      sessionId;
    const data = await sendChatMessage(BACKEND_BASE_URL, {
      sessionId: effectiveSessionId,
      message,
      mode: aiMode
    });
    const responseMode = typeof data.mode === "string" ? data.mode : aiMode;
    writeActiveSessionId(responseMode, data.session_id);
    try {
      await invoke("set_active_session_id", {
        mode: responseMode,
        session_id: data.session_id
      });
    } catch {
      // no-op
    }
    setAiMode(responseMode);
    setSessionId(data.session_id);
    return data.reply;
  }

  async function onSubmit(event) {
    event.preventDefault();
    const text = input.trim();
    if (!text) return;

    setInput("");
    setInputDisabled(true);

    try {
      const reply = await sendMessage(text);
      await pushBubble(reply);
    } catch (error) {
      await pushBubble(`请求失败：${error.message}`);
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
