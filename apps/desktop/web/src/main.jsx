/** Main desktop pet UI entry: avatar interaction, chat input and bubble push flow. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";
import { BACKEND_BASE_URL } from "./config";
import { readActiveSessionId, writeActiveSessionId } from "./session";
import { sendChatMessage } from "./api/chat";

const DRAG_THRESHOLD = 4;
const KEEP_ALIVE_INTERVAL_MS = 350;

function App() {
  const [sessionId, setSessionId] = useState(null);
  const [input, setInput] = useState("");
  const [inputDisabled, setInputDisabled] = useState(false);

  const avatarRef = useRef(null);
  const dragRef = useRef({ pointerDown: false, dragStarted: false, startX: 0, startY: 0 });
  const keepAliveRef = useRef(0);

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

  function avatarAnchorRect() {
    const rect = avatarRef.current?.getBoundingClientRect();
    if (!rect) {
      return null;
    }

    return {
      x: rect.left,
      y: rect.top,
      width: rect.width,
      height: rect.height
    };
  }

  async function toggleMenu() {
    const anchor = avatarAnchorRect();
    if (!anchor) {
      return;
    }

    const activeSessionId = readActiveSessionId() ?? sessionId;
    if (activeSessionId !== sessionId) {
      setSessionId(activeSessionId);
    }

    try {
      await invoke("toggle_avatar_menu", {
        anchor,
        session_id: activeSessionId
      });
    } catch (error) {
      console.error("toggle_avatar_menu failed:", error);
    }
  }

  async function keepMenuAlive() {
    const now = Date.now();
    if (now - keepAliveRef.current < KEEP_ALIVE_INTERVAL_MS) {
      return;
    }

    keepAliveRef.current = now;
    try {
      await invoke("menu_keep_alive");
    } catch {
      // no-op
    }
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

  function onPointerDown(event) {
    dragRef.current.pointerDown = true;
    dragRef.current.dragStarted = false;
    dragRef.current.startX = event.clientX;
    dragRef.current.startY = event.clientY;
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  async function onPointerMove(event) {
    await keepMenuAlive();

    if (!dragRef.current.pointerDown || dragRef.current.dragStarted) return;
    const moved = Math.hypot(
      event.clientX - dragRef.current.startX,
      event.clientY - dragRef.current.startY
    );
    if (moved < DRAG_THRESHOLD) return;

    dragRef.current.dragStarted = true;
    try {
      await invoke("start_window_drag");
    } catch (error) {
      console.error("start_window_drag failed:", error);
    }
  }

  function onPointerEnd() {
    dragRef.current.pointerDown = false;
  }

  async function onAvatarClick() {
    if (dragRef.current.dragStarted) {
      dragRef.current.dragStarted = false;
      return;
    }

    await toggleMenu();
  }

  return (
    <main className="pet-shell">
      <section className="pet-stage">
        <button
          ref={avatarRef}
          id="avatar"
          className="avatar"
          type="button"
          aria-label="桌宠形象"
          onClick={onAvatarClick}
          onDragStart={(e) => e.preventDefault()}
          onMouseEnter={keepMenuAlive}
          onMouseMove={keepMenuAlive}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerEnd}
          onPointerCancel={onPointerEnd}
        >
          <img src="/assets/pet/ava.png" alt="桌宠形象" draggable="false" />
        </button>
      </section>

      <form className="chat-form" onSubmit={onSubmit}>
        <input
          id="chat-input"
          type="text"
          placeholder="说点什么..."
          autoComplete="off"
          required
          value={input}
          disabled={inputDisabled}
          onChange={(e) => setInput(e.target.value)}
        />
      </form>
    </main>
  );
}

createRoot(document.getElementById("root")).render(<App />);
