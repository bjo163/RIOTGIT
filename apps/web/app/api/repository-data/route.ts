import { NextResponse } from "next/server";
import { apiBaseUrl, apiErrorMessage, noStoreHeaders, sessionToken } from "@/lib/server-api";

const SEGMENT_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/;

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  const url = new URL(request.url);
  const resource = url.searchParams.get("resource");
  const owner = url.searchParams.get("owner") ?? "";
  const name = url.searchParams.get("name") ?? "";
  const reference = url.searchParams.get("reference") ?? "HEAD";
  const path = url.searchParams.get("path") ?? "";

  if (!SEGMENT_PATTERN.test(owner) || !SEGMENT_PATTERN.test(name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  if (!["tree", "blob", "commits", "diff"].includes(resource ?? "")) {
    return NextResponse.json({ error: "Choose tree, blob, commits, or diff." }, { status: 400 });
  }
  if (reference.length > 256 || path.length > 2048) {
    return NextResponse.json({ error: "Reference or path is too long." }, { status: 400 });
  }

  const base = url.searchParams.get("base") ?? "main";
  const head = url.searchParams.get("head") ?? "HEAD";
  if (base.length > 256 || head.length > 256) {
    return NextResponse.json({ error: "Diff references are too long." }, { status: 400 });
  }

  const query = resource === "diff"
    ? new URLSearchParams({ base, head })
    : new URLSearchParams({ reference });
  if (resource === "tree" || resource === "blob") query.set("path", path);
  if (resource === "commits") query.set("limit", url.searchParams.get("limit") ?? "30");

  try {
    const upstream = await fetch(
      apiBaseUrl() +
        "/api/v1/repositories/" +
        encodeURIComponent(owner) +
        "/" +
        encodeURIComponent(name) +
        "/" +
        resource +
        "?" +
        query.toString(),
      {
        headers: noStoreHeaders(token),
        cache: "no-store",
      },
    );
    const payload = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok
        ? payload
        : { error: apiErrorMessage(payload, "Repository data could not be loaded.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}
