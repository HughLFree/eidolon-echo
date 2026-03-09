/** Menu window entry: transient buttons and lightweight history panel behavior. */

import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./menu.css";
import { BACKEND_BASE_URL } from "./config";
import { readActiveSessionId, writeActiveSessionId } from "./session";
import { fetchSessionMessages } from "./api/chat";

const AUTO_HIDE_MS = 5000;
const FADE_MS = 200;
const HISTORY_PAGE_SIZE = 10;

function MenuApp() {
  const [mode, setMode] = useState("menu");
  const [fading, setFading] = useState(false);
  const [sessionId, setSessionId] = useState(null);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [loadingMoreHistory, setLoadingMoreHistory] = useState(false);
  const [messages, setMessages] = useState([]);
  const [historyCursor, setHistoryCursor] = useState(null);
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [aiMode, setAiMode] = useState("default");

  const hideTimerRef = useRef(null);
  const fadeTimerRef = useRef(null);
  const modeRef = useRef("menu");
  const aiModeRef = useRef("default");
  const historyListRef = useRef(null);

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

  function resetHistoryState() {
    setMessages([]);
    setHistoryCursor(null);
    setHasMoreHistory(false);
    setLoadingMoreHistory(false);
  }

  async function loadHistoryPage({ reset = false } = {}) {
    let effectiveSessionId = sessionId;
    if (typeof effectiveSessionId !== "number") {
      try {
        const sid = await invoke("get_active_session_id", { mode: aiModeRef.current });
        if (typeof sid === "number") {
          effectiveSessionId = sid;
          setSessionId(sid);
        }
      } catch {
        // no-op
      }
    }

    if (typeof effectiveSessionId !== "number") {
      resetHistoryState();
      return;
    }

    if (!reset) {
      if (!hasMoreHistory || loadingMoreHistory) {
        return;
      }
      setLoadingMoreHistory(true);
    }

    const beforeId = reset ? null : historyCursor;
    const data = await fetchSessionMessages(BACKEND_BASE_URL, {
      sessionId: effectiveSessionId,
      limit: HISTORY_PAGE_SIZE,
      mode: aiModeRef.current,
      beforeId
    });
    const batch = Array.isArray(data.messages) ? data.messages : [];
    const nextCursor = batch.length ? batch[batch.length - 1]?.id ?? null : null;
    const hasMore = batch.length >= HISTORY_PAGE_SIZE;

    if (reset) {
      setMessages(batch);
      setHistoryCursor(nextCursor);
      setHasMoreHistory(hasMore);
      return;
    }

    setMessages((prev) => {
      const known = new Set(prev.map((item) => item.id));
      const append = batch.filter((item) => !known.has(item.id));
      return [...prev, ...append];
    });
    setHistoryCursor(nextCursor);
    setHasMoreHistory(hasMore);
  }

  function onHistoryScroll(event) {
    if (modeRef.current !== "history") {
      return;
    }
    const el = event.currentTarget;
    const nearBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 16;
    if (!nearBottom) {
      return;
    }
    void loadHistoryPage({ reset: false }).finally(() => {
      setLoadingMoreHistory(false);
    });
  }

  async function onHistoryClick() {
    setLoadingHistory(true);
    clearTimers();

    try {
      await invoke("open_history_panel");
      resetHistoryState();
      await loadHistoryPage({ reset: true });
      setMode("history");
      modeRef.current = "history";
    } catch (error) {
      setMessages([{ role: "assistant", content: `加载失败：${error.message}` }]);
      setHistoryCursor(null);
      setHasMoreHistory(false);
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
    resetHistoryState();
    setFading(false);
    clearTimers();
  }

  useEffect(() => {
    let unlistenShow;
    let unlistenKeepalive;
    let unlistenAiMode;

    const bootstrap = async () => {
      try {
        const current = await invoke("get_ai_mode");
        if (typeof current?.mode === "string") {
          setAiMode(current.mode);
          aiModeRef.current = current.mode;
          let activeSid = null;
          try {
            activeSid = await invoke("get_active_session_id", { mode: current.mode });
          } catch {
            // no-op
          }
          const fallbackSid = readActiveSessionId(current.mode);
          setSessionId(typeof activeSid === "number" ? activeSid : fallbackSid);
        }
      } catch {
        // no-op
      }

      unlistenShow = await listen("menu:show", async (event) => {
        const sid = event.payload?.sessionId ?? event.payload?.session_id ?? null;
        const next = typeof sid === "number" ? sid : readActiveSessionId(aiModeRef.current);
        setSessionId(next);
        if (typeof next === "number") {
          writeActiveSessionId(aiModeRef.current, next);
          try {
            await invoke("set_active_session_id", {
              mode: aiModeRef.current,
              session_id: next
            });
          } catch {
            // no-op
          }
        }
        setMode("menu");
        modeRef.current = "menu";
        setFading(false);
        resetHistoryState();
        resetMenuCountdown();
      });

      unlistenKeepalive = await listen("menu:keepalive", () => {
        resetMenuCountdown();
      });

      unlistenAiMode = await listen("ai:mode-changed", async (event) => {
        const nextMode = event.payload?.mode;
        if (typeof nextMode === "string") {
          setAiMode(nextMode);
          aiModeRef.current = nextMode;
          let activeSid = null;
          try {
            activeSid = await invoke("get_active_session_id", { mode: nextMode });
          } catch {
            // no-op
          }
          const fallbackSid = readActiveSessionId(nextMode);
          setSessionId(typeof activeSid === "number" ? activeSid : fallbackSid);
          resetHistoryState();
        }
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
      if (unlistenAiMode) {
        unlistenAiMode();
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
          <div className="history-mini-list" ref={historyListRef} onScroll={onHistoryScroll}>
            {historyContent}
            {loadingMoreHistory ? <p className="history-empty">加载中...</p> : null}
            {!hasMoreHistory && messages.length ? <p className="history-empty">没有更多了</p> : null}
          </div>
        </section>
      )}
    </main>
  );
}

createRoot(document.getElementById("menu-root")).render(<MenuApp />);
