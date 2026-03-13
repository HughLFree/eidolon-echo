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

export async function listAiProviders(baseUrl, { withDisabled = true } = {}) {
  const query = toQuery({ with_disabled: withDisabled });
  const response = await fetch(`${baseUrl}/api/ai-providers${query ? `?${query}` : ""}`);
  return await parseJsonOrThrow(response);
}

export async function createAiProvider(baseUrl, payload) {
  const response = await fetch(`${baseUrl}/api/ai-providers`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
  return await parseJsonOrThrow(response);
}

export async function updateAiProvider(baseUrl, id, payload) {
  const response = await fetch(`${baseUrl}/api/ai-providers/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
  return await parseJsonOrThrow(response);
}

export async function listProfiles(baseUrl, { mode } = {}) {
  const query = toQuery({ mode });
  const response = await fetch(`${baseUrl}/api/profiles${query ? `?${query}` : ""}`);
  return await parseJsonOrThrow(response);
}

export async function createProfile(baseUrl, payload) {
  const response = await fetch(`${baseUrl}/api/profiles`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
  return await parseJsonOrThrow(response);
}

export async function updateProfile(baseUrl, id, payload) {
  const response = await fetch(`${baseUrl}/api/profiles/${encodeURIComponent(id)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload)
  });
  return await parseJsonOrThrow(response);
}
