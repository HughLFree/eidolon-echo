/** Shared HTTP client helpers for chat and message history endpoints. */

async function parseJsonOrThrow(response) {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `HTTP ${response.status}`);
  }
  return await response.json();
}

export async function sendChatMessage(baseUrl, { sessionId, message, mode }) {
  const response = await fetch(`${baseUrl}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      session_id: sessionId,
      message,
      mode: mode || "default"
    })
  });

  return await parseJsonOrThrow(response);
}

export async function fetchSessionMessages(baseUrl, { sessionId, limit = 50, mode = "default", beforeId = null }) {
  const params = new URLSearchParams({
    limit: String(limit),
    mode: mode || "default"
  });
  if (typeof beforeId === "number" && Number.isFinite(beforeId)) {
    params.set("before_id", String(beforeId));
  }
  const response = await fetch(`${baseUrl}/api/sessions/${sessionId}/messages?${params.toString()}`);
  return await parseJsonOrThrow(response);
}
