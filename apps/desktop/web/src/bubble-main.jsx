/** Bubble window entry: displays assistant text with timed auto-hide behavior. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./bubble.css";

const BUBBLE_AUTO_HIDE_MS = 15000;

function BubbleApp() {
  const [text, setText] = useState("");
  const timerRef = useRef(null);
  const bubbleRef = useRef(null);
  const previousTextRef = useRef("");
  const stickToBottomRef = useRef(true);

  function resetAutoHideTimer() {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }
    timerRef.current = setTimeout(() => {
      setText("");
    }, BUBBLE_AUTO_HIDE_MS);
  }

  useEffect(() => {
    window.__desktopAiSetBubble = (value) => {
      const parsed = String(value ?? "").trim();
      if (!parsed) {
        setText("");
        return;
      }

      setText(parsed);
      resetAutoHideTimer();
    };

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      delete window.__desktopAiSetBubble;
    };
  }, []);

  useEffect(() => {
    if (text && bubbleRef.current) {
      const el = bubbleRef.current;
      const previous = previousTextRef.current;
      const isAppend = previous && text.startsWith(previous);

      if (!isAppend) {
        el.scrollTop = 0;
        stickToBottomRef.current = true;
      } else if (stickToBottomRef.current) {
        el.scrollTop = el.scrollHeight;
      }
    }
    previousTextRef.current = text;
  }, [text]);

  useEffect(() => {
    invoke("set_bubble_interactive", { interactive: Boolean(text) }).catch((error) => {
      console.error("set_bubble_interactive failed:", error);
    });
  }, [text]);

  function keepAliveWhileInteracting() {
    if (!text) {
      return;
    }
    resetAutoHideTimer();
  }

  function onBubbleScroll() {
    if (!bubbleRef.current) {
      return;
    }
    const el = bubbleRef.current;
    const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    stickToBottomRef.current = distanceToBottom <= 12;
    keepAliveWhileInteracting();
  }

  return (
    <main className="bubble-root">
      <section
        id="bubble"
        ref={bubbleRef}
        className={`bubble ${text ? "show" : ""}`}
        aria-live="polite"
        onWheel={keepAliveWhileInteracting}
        onScroll={onBubbleScroll}
        onMouseMove={keepAliveWhileInteracting}
      >
        {text}
      </section>
    </main>
  );
}

createRoot(document.getElementById("bubble-root")).render(<BubbleApp />);
