import { NextResponse } from "next/server";
import { SESSION_COOKIE, apiBaseUrl, noStoreHeaders, sessionToken } from "@/lib/server-api";

export async function POST(request: Request) {
  const token = sessionToken(request);
  if (token) {
    try {
      await fetch(apiBaseUrl() + "/api/v1/auth/logout", {
        method: "POST",
        headers: noStoreHeaders(token),
        cache: "no-store",
      });
    } catch {
      // Clear the browser cookie even if the API is unavailable.
    }
  }

  const response = new NextResponse(null, { status: 204 });
  response.cookies.delete(SESSION_COOKIE);
  return response;
}
