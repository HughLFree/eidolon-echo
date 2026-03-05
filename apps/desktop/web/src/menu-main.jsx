/** Menu window entry: transient buttons and lightweight history panel behavior. */

import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./menu.css";
import { BACKEND_BASE_URL } from "./config";
import { writeActiveSessionId } from "./session";
import { fetchSessionMessages } from "./api/chat";

const AUTO_HIDE_MS = 5000;
const FADE_MS = 200;

function MenuApp() {
  const [mode, setMode] = useState("menu");
  const [fading, setFading] = useState(false);
  const [sessionId, setSessionId] = useState(null);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [messages, setMessages] = useState([]);

  const hideTimerRef = useRef(null);
  const fadeTimerRef = useRef(null);
  const modeRef = useRef("menu");

  function clearTimers() {
    if (hideTimerRef.current) {
      clearTimeout(hideTimerRef.current);
      hideTimerRef.current = null;
    }
    if (fadeTimerRef.current) {
      clearTimeout(fadeTimerRef.current);
      fadeTimerRef.current = null;
    }
  }

  function resetMenuCountdown() {
    if (modeRef.current !== "menu") {
      return;
    }

    clearTimers();
    setFading(false);

    hideTimerRef.current = setTimeout(() => {
      setFading(true);
      fadeTimerRef.current = setTimeout(async () => {
        try {
          await invoke("hide_menu_window");
        } catch {
          // no-op
        }
        setFading(false);
      }, FADE_MS);
    }, AUTO_HIDE_MS);
  }

  async function loadRecentHistory() {
    if (!sessionId) {
      setMessages([]);
      return;
    }

    const data = await fetchSessionMessages(BACKEND_BASE_URL, {
      sessionId,
      limit: 10
    });
    setMessages(data.messages || []);
  }

  async function onHistoryClick() {
    setLoadingHistory(true);
    clearTimers();

    try {
      await invoke("open_history_panel");
      await loadRecentHistory();
      setMode("history");
      modeRef.current = "history";
    } catch (error) {
      setMessages([{ role: "assistant", content: `加载失败：${error.message}` }]);
      setMode("history");
      modeRef.current = "history";
    } finally {
      setLoadingHistory(false);
      setFading(false);
    }
  }

  async function onSettingsClick() {
    try {
      await invoke("set_bubble_text", { text: "设置功能开发中" });
    } catch {
      // no-op
    }
  }

  async function onCloseHistory() {
    try {
      await invoke("hide_menu_window");
    } catch {
      // no-op
    }
    setMode("menu");
    modeRef.current = "menu";
    setMessages([]);
    setFading(false);
    clearTimers();
  }

  useEffect(() => {
    let unlistenShow;
    let unlistenKeepalive;

    const bootstrap = async () => {
      unlistenShow = await listen("menu:show", (event) => {
        const sid = event.payload?.sessionId ?? event.payload?.session_id ?? null;
        const next = typeof sid === "number" ? sid : null;
        setSessionId(next);
        writeActiveSessionId(next);
        setMode("menu");
        modeRef.current = "menu";
        setFading(false);
        setMessages([]);
        resetMenuCountdown();
      });

      unlistenKeepalive = await listen("menu:keepalive", () => {
        resetMenuCountdown();
      });
    };

    void bootstrap();

    return () => {
      clearTimers();
      if (unlistenShow) {
        unlistenShow();
      }
      if (unlistenKeepalive) {
        unlistenKeepalive();
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

  const historyContent = useMemo(() => {
    if (!messages.length) {
      return <p className="history-empty">暂无历史消息</p>;
    }

    return messages.map((message, index) => (
      <article className="history-item" key={`${message.id ?? index}-${index}`}>
        <p className="history-role">{message.role === "assistant" ? "桌宠" : "你"}</p>
        <p className="history-text">{message.content}</p>
      </article>
    ));
  }, [messages]);

  return (
    <main className="menu-root" onMouseEnter={resetMenuCountdown} onMouseMove={resetMenuCountdown}>
      {mode === "menu" ? (
        <section className={`menu-capsule ${fading ? "fading" : ""}`}>
          <button className="capsule-btn" type="button" onClick={onHistoryClick}>
            {loadingHistory ? "..." : "h"}
          </button>
          <button className="capsule-btn" type="button" onClick={onSettingsClick}>
            s
          </button>
        </section>
      ) : (
        <section className="history-panel-mini">
          <header className="history-mini-header">
            <strong>最近消息</strong>
            <button className="history-mini-close" type="button" onClick={onCloseHistory}>
              ×
            </button>
          </header>
          <div className="history-mini-list">{historyContent}</div>
        </section>
      )}
    </main>
  );
}

createRoot(document.getElementById("menu-root")).render(<MenuApp />);
