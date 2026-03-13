/** Bubble window entry: displays assistant text with timed auto-hide behavior. */

import React, { useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./bubble.css";

function BubbleApp() {
  const [text, setText] = useState("");
  const timerRef = useRef(null);
  const bubbleRef = useRef(null);

  useEffect(() => {
    window.__desktopAiSetBubble = (value) => {
      const parsed = String(value ?? "").trim();
      if (!parsed) {
        setText("");
        return;
      }

      setText(parsed);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      timerRef.current = setTimeout(() => {
        setText("");
      }, 12000);
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
      bubbleRef.current.scrollTop = 0;
    }
  }, [text]);

  useEffect(() => {
    invoke("set_bubble_interactive", { interactive: Boolean(text) }).catch((error) => {
      console.error("set_bubble_interactive failed:", error);
    });
  }, [text]);

  return (
    <main className="bubble-root">
      <section
        id="bubble"
        ref={bubbleRef}
        className={`bubble ${text ? "show" : ""}`}
        aria-live="polite"
      >
        {text}
      </section>
    </main>
  );
}

createRoot(document.getElementById("bubble-root")).render(<BubbleApp />);
