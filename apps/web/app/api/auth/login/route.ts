import { NextResponse } from "next/server";
import { SESSION_COOKIE, apiBaseUrl, apiErrorMessage } from "@/lib/server-api";

type LoginPayload = { username?: unknown; password?: unknown };
type LoginResult = { access_token?: unknown; expires_at?: unknown; user?: unknown };

export async function POST(request: Request) {
  let input: LoginPayload;
  try {
    input = (await request.json()) as LoginPayload;
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }

  if (
    typeof input.username !== "string" ||
    typeof input.password !== "string" ||
    input.username.trim().length === 0 ||
    input.password.length === 0 ||
    input.username.length > 100 ||
    input.password.length > 1024
  ) {
    return NextResponse.json({ error: "Enter a valid username and password." }, { status: 400 });
  }

  try {
    const upstream = await fetch(apiBaseUrl() + "/api/v1/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ username: input.username.trim(), password: input.password }),
      cache: "no-store",
    });
    const payload = (await upstream.json().catch(() => null)) as LoginResult | null;

    if (!upstream.ok || !payload || typeof payload.access_token !== "string") {
      return NextResponse.json(
        { error: apiErrorMessage(payload, "Sign-in failed. Check your credentials and try again.") },
        { status: upstream.ok ? 502 : upstream.status },
      );
    }

    const expiresAt = typeof payload.expires_at === "number"
      ? payload.expires_at
      : Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60;
    const response = NextResponse.json({ user: payload.user ?? null, expires_at: expiresAt });
    response.cookies.set(SESSION_COOKIE, payload.access_token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === "production",
      sameSite: "lax",
      path: "/",
      maxAge: Math.max(0, expiresAt - Math.floor(Date.now() / 1000)),
    });
    return response;
  } catch {
    return NextResponse.json(
      { error: "R!OTGIT API is unavailable. Check the API service and try again." },
      { status: 502 },
    );
  }
}
