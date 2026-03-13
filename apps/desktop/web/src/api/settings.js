/** HTTP helpers for settings-related provider/profile CRUD endpoints. */

async function parseJsonOrThrow(response) {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `HTTP ${response.status}`);
  }
  return await response.json();
}

function toQuery(params) {
  const query = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value === undefined || value === null || value === "") {
      return;
    }
    query.set(key, String(value));
  });
  return query.toString();
}

async function requestJson(url, options) {
  try {
    const response = await fetch(url, options);
    return await parseJsonOrThrow(response);
  } catch (error) {
    const message = String(error?.message || error || "").trim();
    throw new Error(`request failed (${url}): ${message || "unknown error"}`);
  }
}

function isNetworkLikeError(error) {
  const raw = String(error?.message || error || "").toLowerCase();
  return (
    raw.includes("load failed")
    || raw.includes("failed to fetch")
    || raw.includes("networkerror")
    || raw.includes("connection refused")
    || raw.includes("connect")
    || raw.includes("timed out")
  );
}

function swapLoopbackHost(url) {
  if (url.includes("127.0.0.1")) {
    return url.replace("127.0.0.1", "localhost");
  }
  if (url.includes("localhost")) {
    return url.replace("localhost", "127.0.0.1");
  }
  return null;
}

async function requestJsonWithLoopbackFallback(url, options) {
  try {
    return await requestJson(url, options);
  } catch (error) {
    const fallbackUrl = swapLoopbackHost(url);
    if (!fallbackUrl || !isNetworkLikeError(error)) {
      throw error;
    }
    return await requestJson(fallbackUrl, options);
  }
}

export async function listAiProviders(baseUrl, { withDisabled = true } = {}) {
  const query = toQuery({ with_disabled: withDisabled });
  const url = `${baseUrl}/api/ai-providers${query ? `?${query}` : ""}`;
  try {
    return await requestJsonWithLoopbackFallback(url);
  } catch (error) {
    if (!withDisabled) {
      throw error;
    }

    // Fallback for environments where boolean query parsing differs.
    return await requestJsonWithLoopbackFallback(`${baseUrl}/api/ai-providers`);
  }
}

export async function createAiProvider(baseUrl, payload) {
  const url = `${baseUrl}/api/ai-providers`;
  return await requestJsonWithLoopbackFallback(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export async function updateAiProvider(baseUrl, id, payload) {
  const url = `${baseUrl}/api/ai-providers/${encodeURIComponent(id)}`;
  return await requestJsonWithLoopbackFallback(url, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export async function listProfiles(baseUrl, { mode } = {}) {
  const query = toQuery({ mode });
  const url = `${baseUrl}/api/profiles${query ? `?${query}` : ""}`;
  return await requestJsonWithLoopbackFallback(url);
}

export async function createProfile(baseUrl, payload) {
  const url = `${baseUrl}/api/profiles`;
  return await requestJsonWithLoopbackFallback(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}

export async function updateProfile(baseUrl, id, payload) {
  const url = `${baseUrl}/api/profiles/${encodeURIComponent(id)}`;
  return await requestJsonWithLoopbackFallback(url, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
}
