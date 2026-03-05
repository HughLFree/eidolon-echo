/** Shared HTTP client helpers for chat and message history endpoints. */

async function parseJsonOrThrow(response) {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `HTTP ${response.status}`);
  }
  return await response.json();
}

export async function sendChatMessage(baseUrl, { sessionId, message }) {
  const response = await fetch(`${baseUrl}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ session_id: sessionId, message })
  });

  return await parseJsonOrThrow(response);
}

export async function fetchSessionMessages(baseUrl, { sessionId, limit = 50 }) {
  const response = await fetch(`${baseUrl}/api/sessions/${sessionId}/messages?limit=${limit}`);
  return await parseJsonOrThrow(response);
}

export function pickLatestAssistantOrLast(messages) {
  const list = Array.isArray(messages) ? messages : [];
  const latestAssistant = [...list].reverse().find((m) => m.role === "assistant");
  return latestAssistant ?? list[list.length - 1] ?? null;
}
