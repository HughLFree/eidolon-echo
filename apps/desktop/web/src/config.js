/** Frontend runtime config sourced from Vite env with sensible local defaults. */

export const BACKEND_BASE_URL =
  import.meta.env.VITE_BACKEND_BASE_URL || "http://127.0.0.1:3001";
