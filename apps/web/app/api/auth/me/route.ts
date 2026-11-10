import { NextResponse } from "next/server";
import {
  SESSION_COOKIE,
  apiBaseUrl,
  apiErrorMessage,
  noStoreHeaders,
  sessionToken,
} from "@/lib/server-api";

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  try {
    const upstream = await fetch(apiBaseUrl() + "/api/v1/auth/me", {
      headers: noStoreHeaders(token),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    const response = NextResponse.json(
      upstream.ok ? payload : { error: apiErrorMessage(payload, "Your session is no longer valid.") },
      { status: upstream.status },
    );
    if (upstream.status === 401) response.cookies.delete(SESSION_COOKIE);
    return response;
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}
