/** Shared HTTP client helpers for chat and message history endpoints. */

async function parseJsonOrThrow(response) {
  if (!response.ok) {
    const text = await response.text();
    const message = normalizeErrorText(text);
    throw new Error(message || `HTTP ${response.status}`);
  }
  return await response.json();
}

function normalizeErrorText(rawText) {
  const text = String(rawText || "").trim();
  if (!text) {
    return "";
  }

  try {
    const parsed = JSON.parse(text);
    if (typeof parsed === "string") {
      return parsed;
    }
    if (parsed && typeof parsed === "object") {
      if (typeof parsed.message === "string" && parsed.message.trim()) {
        return parsed.message.trim();
      }
      if (parsed.error && typeof parsed.error.message === "string" && parsed.error.message.trim()) {
        return parsed.error.message.trim();
      }
    }
  } catch {
    // ignore parse errors and keep original text
  }

  return text;
}

export async function sendChatMessage(baseUrl, { message, mode }) {
  const response = await fetch(`${baseUrl}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      message,
      mode: mode || "default"
    })
  });

  return await parseJsonOrThrow(response);
}

export async function sendChatMessageStream(baseUrl, { message, mode, onDelta }) {
  const requestBody = {
    message,
    mode: mode || "default"
  };

  async function fallbackToNonStream() {
    const data = await sendChatMessage(baseUrl, requestBody);
    if (typeof onDelta === "function") {
      await onDelta(data.reply || "");
    }
    return data;
  }

  try {
    const response = await fetch(`${baseUrl}/api/chat/stream`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(requestBody)
    });

    if (!response.ok) {
      // Fallback keeps chat available when stream route is not ready/restarted yet.
      return await fallbackToNonStream();
    }

    const reader = response.body?.getReader();
    if (!reader) {
      return await fallbackToNonStream();
    }

    const decoder = new TextDecoder("utf-8");
    let reply = "";
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      const chunk = decoder.decode(value, { stream: true });
      if (!chunk) {
        continue;
      }
      reply += chunk;
      if (typeof onDelta === "function") {
        await onDelta(reply);
      }
    }

    const tail = decoder.decode();
    if (tail) {
      reply += tail;
      if (typeof onDelta === "function") {
        await onDelta(reply);
      }
    }

    return {
      mode: mode || "default",
      reply
    };
  } catch (_error) {
    return await fallbackToNonStream();
  }
}

export async function fetchConversationMessages(baseUrl, { limit = 50, mode = "default", beforeId = null }) {
  const params = new URLSearchParams({
    limit: String(limit),
    mode: mode || "default"
  });
  if (typeof beforeId === "number" && Number.isFinite(beforeId)) {
    params.set("before_id", String(beforeId));
  }
  const response = await fetch(`${baseUrl}/api/messages?${params.toString()}`);
  return await parseJsonOrThrow(response);
}

export async function fetchBackendHealth(baseUrl, { mode = null } = {}) {
  const query = new URLSearchParams();
  if (typeof mode === "string" && mode.trim()) {
    query.set("mode", mode.trim());
  }
  const suffix = query.toString();
  const response = await fetch(`${baseUrl}/api/health${suffix ? `?${suffix}` : ""}`);
  return await parseJsonOrThrow(response);
}
