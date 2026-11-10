export const SESSION_COOKIE = "riotgit_session";

export function apiBaseUrl(): string {
  const configured = process.env.RIOTGIT_API_URL ?? "http://127.0.0.1:8787";
  return configured.replace(/\/+$/, "");
}

export function sessionToken(request: Request): string | null {
  const cookieHeader = request.headers.get("cookie");
  if (!cookieHeader) return null;

  for (const part of cookieHeader.split(";")) {
    const [name, ...valueParts] = part.trim().split("=");
    if (name === SESSION_COOKIE) {
      try {
        return decodeURIComponent(valueParts.join("="));
      } catch {
        return null;
      }
    }
  }
  return null;
}

export function apiErrorMessage(payload: unknown, fallback: string): string {
  if (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof payload.error === "object" &&
    payload.error !== null &&
    "message" in payload.error &&
    typeof payload.error.message === "string"
  ) {
    return payload.error.message;
  }
  return fallback;
}

export function noStoreHeaders(token?: string): HeadersInit {
  const headers: Record<string, string> = { Accept: "application/json" };
  if (token) headers.Authorization = "Bearer " + token;
  return headers;
}
