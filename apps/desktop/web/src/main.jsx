/** Main desktop pet UI entry: avatar interaction and menu toggles. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";

const DRAG_THRESHOLD = 4;
const KEEP_ALIVE_INTERVAL_MS = 350;

function App() {
  const avatarRef = useRef(null);
  const dragRef = useRef({ pointerDown: false, dragStarted: false, startX: 0, startY: 0 });
  const keepAliveRef = useRef(0);
  const [avatarPng, setAvatarPng] = useState("/assets/pet/default-avatar.png");
  const [aiMode, setAiMode] = useState("default");

  useEffect(() => {
    void invoke("set_overlay_always_on_top", {
      alwaysOnTop: true
    }).catch(() => {
      // no-op
    });
  }, []);
  useEffect(() => {
    let unlistenMode;

    const bootstrap = async () => {
      try {
        const current = await invoke("get_ai_mode");
        if (typeof current?.mode === "string") {
          setAiMode(current.mode);
        }
        if (typeof current?.avatarPng === "string") {
          setAvatarPng(current.avatarPng);
        }
      } catch {
        // no-op
      }

      unlistenMode = await listen("ai:mode-changed", (event) => {
        const nextMode = event.payload?.mode;
        if (typeof nextMode === "string") {
          setAiMode(nextMode);
        }
        const avatar = event.payload?.avatarPng;
        if (typeof avatar === "string") {
          setAvatarPng(avatar);
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

    try {
      await invoke("toggle_avatar_menu", { anchor });
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
    <main className="pet-shell avatar-only">
      <section className="pet-stage">
        <button
          ref={avatarRef}
          id="avatar"
          className="avatar"
          type="button"
          aria-label="伙伴形象"
          onClick={onAvatarClick}
          onDragStart={(e) => e.preventDefault()}
          onMouseEnter={keepMenuAlive}
          onMouseMove={keepMenuAlive}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerEnd}
          onPointerCancel={onPointerEnd}
        >
          <img src={avatarPng} alt="伙伴形象" draggable="false" />
        </button>
      </section>
    </main>
  );
}

createRoot(document.getElementById("root")).render(<App />);
