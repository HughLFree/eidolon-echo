/** Chat window entry: dedicated input box for sending messages and showing bubble reply. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./chat.css";
import { BACKEND_BASE_URL } from "./config";
import { sendChatMessageStream } from "./api/chat";

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
