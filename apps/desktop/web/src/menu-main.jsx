/** Quick panel window entry: lightweight history access for avatar popup panel. */

import React, { useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./menu.css";
import { BACKEND_BASE_URL } from "./config";
import { fetchConversationMessages } from "./api/chat";

const AUTO_HIDE_MS = 5000;
const FADE_MS = 200;
const HISTORY_PAGE_SIZE = 10;

function toUserFriendlyLoadMessage(error) {
  const raw = String(error?.message || error || "").trim();
  const lower = raw.toLowerCase();

  if (
    lower.includes("backend startup failed")
    || lower.includes("unable to locate backend sidecar binary")
    || lower.includes("failed to spawn backend process")
    || lower.includes("timed out")
    || lower.includes("exited during startup")
  ) {
    return "本地后端启动失败。请完全退出应用后重试；若仍失败，请在终端先运行后端查看具体错误。";
  }

  if (
    lower.includes("failed to fetch")
    || lower.includes("load failed")
    || lower.includes("networkerror")
    || lower.includes("connection refused")
    || lower.includes("connect")
    || lower.includes("timed out")
  ) {
    return "无法加载历史消息。请先确认应用已正常启动；如果是首次使用，请在“设置 -> API 设置”完成模型配置。";
  }

  if (
    lower.includes("401")
    || lower.includes("403")
    || lower.includes("unauthorized")
    || lower.includes("authentication")
    || lower.includes("invalid api key")
    || lower.includes("api key")
    || lower.includes("authentication fails")
    || lower.includes("invalid_request_error")
  ) {
    return "无法加载历史消息。模型鉴权失败，请在“设置 -> API 设置”检查 api_key / base_url / model_name。";
  }

  return raw ? `加载失败：${raw}` : "加载失败，请稍后重试。";
}

function MenuApp() {
  const [mode, setMode] = useState("panel");
  const [fading, setFading] = useState(false);
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [loadingMoreHistory, setLoadingMoreHistory] = useState(false);
  const [selectedMessage, setSelectedMessage] = useState(null);
  const [messages, setMessages] = useState([]);
  const [historyCursor, setHistoryCursor] = useState(null);
  const [hasMoreHistory, setHasMoreHistory] = useState(false);
  const [aiMode, setAiMode] = useState("default");

  const hideTimerRef = useRef(null);
  const fadeTimerRef = useRef(null);
  const modeRef = useRef("panel");
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
    if (modeRef.current !== "panel") {
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
    setSelectedMessage(null);
    setMessages([]);
    setHistoryCursor(null);
    setHasMoreHistory(false);
    setLoadingMoreHistory(false);
  }

  async function syncModeFromTauri() {
    try {
      const current = await invoke("get_ai_mode");
      if (typeof current?.mode === "string") {
        setAiMode(current.mode);
        aiModeRef.current = current.mode;
      }
    } catch {
      // no-op
    }
    return aiModeRef.current;
  }

  async function loadHistoryPage({ reset = false } = {}) {
    if (!reset) {
      if (!hasMoreHistory || loadingMoreHistory) {
        return;
      }
      setLoadingMoreHistory(true);
    }

    const beforeId = reset ? null : historyCursor;
    const data = await fetchConversationMessages(BACKEND_BASE_URL, {
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
    if (modeRef.current !== "history" || selectedMessage) {
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

  function onSelectHistoryMessage(message) {
    setSelectedMessage(message);
  }

  function onBackToHistoryList() {
    setSelectedMessage(null);
  }

  async function onHistoryClick() {
    setLoadingHistory(true);
    clearTimers();

    try {
      await ensureBackendReady();
      await syncModeFromTauri();
      await invoke("open_history_panel");
      resetHistoryState();
      await loadHistoryPage({ reset: true });
      setMode("history");
      modeRef.current = "history";
    } catch (error) {
      setMessages([{ role: "assistant", content: toUserFriendlyLoadMessage(error) }]);
      setHistoryCursor(null);
      setHasMoreHistory(false);
      setMode("history");
      modeRef.current = "history";
    } finally {
      setLoadingHistory(false);
      setFading(false);
    }
  }

  async function onCloseHistory() {
    try {
      await invoke("hide_menu_window");
    } catch {
      // no-op
    }
    setMode("panel");
    modeRef.current = "panel";
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
        await syncModeFromTauri();
      } catch {
        // no-op
      }

      unlistenShow = await listen("menu:show", async () => {
        await syncModeFromTauri();
        setMode("panel");
        modeRef.current = "panel";
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
  const historyContent = useMemo(() => {
    if (!messages.length) {
      return <p className="history-empty">暂无历史消息</p>;
    }

    return messages.map((message, index) => (
      <button
        className="history-item history-item-btn"
        key={`${message.id ?? index}-${index}`}
        onClick={() => onSelectHistoryMessage(message)}
        type="button"
      >
        <p className="history-role">{message.role === "assistant" ? "伙伴" : "你"}</p>
        <p className="history-text">{message.content}</p>
      </button>
    ));
  }, [messages]);

  return (
    <main className="menu-root" onMouseEnter={resetMenuCountdown} onMouseMove={resetMenuCountdown}>
      {mode === "panel" ? (
        <section className={`menu-capsule ${fading ? "fading" : ""}`}>
          <button className="capsule-btn" type="button" onClick={onHistoryClick}>
            {loadingHistory ? "..." : "历"}
          </button>
        </section>
      ) : (
        <section className="history-panel-mini">
          <header className="history-mini-header">
            {selectedMessage ? (
              <button className="history-mini-back" onClick={onBackToHistoryList} type="button">
                ←
              </button>
            ) : (
              <span aria-hidden="true" className="history-mini-spacer" />
            )}
            <strong>{selectedMessage ? "消息详情" : "最近消息"}</strong>
            <button className="history-mini-close" type="button" onClick={onCloseHistory}>
              ×
            </button>
          </header>
          {selectedMessage ? (
            <article className="history-detail">
              <p className="history-role">{selectedMessage.role === "assistant" ? "伙伴" : "你"}</p>
              <p className="history-text-full">{selectedMessage.content}</p>
            </article>
          ) : (
            <div className="history-mini-list" ref={historyListRef} onScroll={onHistoryScroll}>
              {historyContent}
              {loadingMoreHistory ? <p className="history-empty">加载中...</p> : null}
              {!hasMoreHistory && messages.length ? <p className="history-empty">没有更多了</p> : null}
            </div>
          )}
        </section>
      )}
    </main>
  );
}

async function ensureBackendReady() {
  try {
    await invoke("ensure_backend_ready");
  } catch (error) {
    const message = String(error?.message || error || "").trim();
    throw new Error(message || "backend startup failed");
  }
}

createRoot(document.getElementById("menu-root")).render(<MenuApp />);
