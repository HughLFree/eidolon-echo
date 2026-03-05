/** Dialog window entry: compact chat panel for quick conversation and latest output. */

import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./dialog.css";
import { BACKEND_BASE_URL } from "./config";
import {
  normalizeSessionId,
  readActiveSessionId,
  writeActiveSessionId
} from "./session";
import {
  fetchSessionMessages,
  pickLatestAssistantOrLast,
  sendChatMessage
} from "./api/chat";

function DialogApp() {
  const [sessionId, setSessionId] = useState(null);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const [output, setOutput] = useState("准备好对话了。");

  async function loadLatestOutput(targetSessionId) {
    try {
      if (!targetSessionId) {
        setOutput("准备好对话了。");
        return;
      }

      const data = await fetchSessionMessages(BACKEND_BASE_URL, {
        sessionId: targetSessionId,
        limit: 20
      });
      const latest = pickLatestAssistantOrLast(data.messages);
      setOutput(latest?.content || "准备好对话了。");
    } catch (error) {
      setOutput(`读取失败：${error.message}`);
    }
  }

  async function onSubmit(event) {
    event.preventDefault();
    const text = input.trim();
    if (!text || sending) {
      return;
    }

    setInput("");
    setSending(true);

    try {
      const sharedSessionId = readActiveSessionId();
      const effectiveSessionId = sharedSessionId ?? sessionId;
      if (sharedSessionId !== null && sharedSessionId !== sessionId) {
        setSessionId(sharedSessionId);
      }

      const data = await sendChatMessage(BACKEND_BASE_URL, {
        sessionId: effectiveSessionId,
        message: text
      });
      const newSessionId = normalizeSessionId(data.session_id);
      setSessionId(newSessionId);
      writeActiveSessionId(newSessionId);
      setOutput(data.reply || "（空响应）");
    } catch (error) {
      setOutput(`请求失败：${error.message}`);
    } finally {
      setSending(false);
    }
  }

  useEffect(() => {
    const onKeyDown = (event) => {
      if (event.key === "Escape") {
        void invoke("overlay_hide_chat_panel");
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  useEffect(() => {
    const bootstrap = async () => {
      const sid = readActiveSessionId();
      setSessionId(sid);
      await loadLatestOutput(sid);
    };

    void bootstrap();
  }, []);

  return (
    <main className="dialog-shell">
      <form className="dialog-input" onSubmit={onSubmit}>
        <input
          type="text"
          placeholder="输入消息..."
          autoComplete="off"
          value={input}
          disabled={sending}
          onChange={(e) => setInput(e.target.value)}
        />
        <button type="submit" disabled={sending || !input.trim()}>
          发送
        </button>
      </form>

      <section className="dialog-content">
        <p className="output">{output}</p>
      </section>
    </main>
  );
}

createRoot(document.getElementById("dialog-root")).render(<DialogApp />);
