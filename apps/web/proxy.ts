import { NextRequest, NextResponse } from "next/server";

const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

/**
 * The web application uses an HttpOnly cookie as its browser session.
 * Require a same-origin Origin header for all mutating /api requests so
 * cross-site forms and credentialed fetches cannot ride that cookie.
 *
 * Direct Git/API clients call the Rust API and are not routed through here.
 */
export function proxy(request: NextRequest) {
  if (!MUTATING_METHODS.has(request.method)) {
    return NextResponse.next();
  }

  const originHeader = request.headers.get("origin");
  const forwardedHost = request.headers.get("x-forwarded-host");
  const host = (forwardedHost ?? request.headers.get("host") ?? "").split(",")[0].trim();
  const forwardedProto = request.headers.get("x-forwarded-proto");
  const protocol = (forwardedProto ?? request.nextUrl.protocol.replace(":", "")).split(",")[0].trim();

  if (!originHeader || !host || !protocol) {
    return NextResponse.json({ error: "A same-origin request is required." }, { status: 403 });
  }

  let origin: URL;
  try {
    origin = new URL(originHeader);
  } catch {
    return NextResponse.json({ error: "The request origin is invalid." }, { status: 403 });
  }

  const expectedOrigin = protocol + "://" + host.toLowerCase();
  if (origin.origin.toLowerCase() !== expectedOrigin) {
    return NextResponse.json({ error: "Cross-origin mutations are not allowed." }, { status: 403 });
  }

  if (request.headers.get("sec-fetch-site") === "cross-site") {
    return NextResponse.json({ error: "Cross-site mutations are not allowed." }, { status: 403 });
  }

  return NextResponse.next();
}

export const config = {
  matcher: ["/api/:path*"],
};
