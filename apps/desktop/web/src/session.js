/** Client-local session_id persistence helpers (mode-scoped, localStorage-backed). */

const KEY = "desktop_ai_active_session_ids_v1";
const LEGACY_KEY = "desktop_ai_active_session_id";

export function normalizeSessionId(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function normalizeMode(mode) {
  return mode === "roleplay" ? "roleplay" : "default";
}

function readSessionMap() {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === "object") {
        return parsed;
      }
    }

    // One-time compatibility fallback for old single-session storage.
    const legacyRaw = window.localStorage.getItem(LEGACY_KEY);
    if (!legacyRaw) {
      return {};
    }
    const legacy = normalizeSessionId(Number(legacyRaw));
    if (legacy === null) {
      return {};
    }
    return { default: legacy };
  } catch {
    return {};
  }
}

function writeSessionMap(sessionMap) {
  try {
    if (!sessionMap || Object.keys(sessionMap).length === 0) {
      window.localStorage.removeItem(KEY);
      return;
    }
    window.localStorage.setItem(KEY, JSON.stringify(sessionMap));
  } catch {
    // no-op
  }
}

export function readActiveSessionId(mode = "default") {
  const sessionMap = readSessionMap();
  const normalizedMode = normalizeMode(mode);
  return normalizeSessionId(sessionMap[normalizedMode]);
}

export function writeActiveSessionId(mode, sessionId) {
  const sessionMap = readSessionMap();
  const normalizedMode = normalizeMode(mode);
  const normalizedSessionId = normalizeSessionId(sessionId);

  if (normalizedSessionId === null) {
    delete sessionMap[normalizedMode];
  } else {
    sessionMap[normalizedMode] = normalizedSessionId;
  }

  writeSessionMap(sessionMap);
}
