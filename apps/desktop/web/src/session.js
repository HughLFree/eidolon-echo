/** Client-local session_id persistence helpers (localStorage-backed). */

const KEY = "desktop_ai_active_session_id";

export function normalizeSessionId(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function readActiveSessionId() {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (!raw) return null;
    const parsed = Number(raw);
    return normalizeSessionId(parsed);
  } catch {
    return null;
  }
}

export function writeActiveSessionId(sessionId) {
  const normalized = normalizeSessionId(sessionId);
  try {
    if (normalized === null) {
      window.localStorage.removeItem(KEY);
      return;
    }
    window.localStorage.setItem(KEY, String(normalized));
  } catch {
    // no-op
  }
}
