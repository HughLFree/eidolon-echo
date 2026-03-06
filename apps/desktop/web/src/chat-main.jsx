/** Chat window entry: dedicated input box for sending messages and showing bubble reply. */

import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./chat.css";
import { BACKEND_BASE_URL } from "./config";
import { readActiveSessionId, writeActiveSessionId } from "./session";
import { sendChatMessage } from "./api/chat";

function ChatApp() {
  const [sessionId, setSessionId] = useState(null);
  const [input, setInput] = useState("");
  const [inputDisabled, setInputDisabled] = useState(false);

  useEffect(() => {
    setSessionId(readActiveSessionId());
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
    const effectiveSessionId = readActiveSessionId() ?? sessionId;
    const data = await sendChatMessage(BACKEND_BASE_URL, {
      sessionId: effectiveSessionId,
      message
    });
    setSessionId(data.session_id);
    writeActiveSessionId(data.session_id);
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
