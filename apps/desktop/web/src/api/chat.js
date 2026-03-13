/** Shared HTTP client helpers for chat and message history endpoints. */

async function parseJsonOrThrow(response) {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `HTTP ${response.status}`);
  }
  return await response.json();
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
