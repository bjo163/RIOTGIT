import { NextResponse } from "next/server";
import {
  apiBaseUrl,
  apiErrorMessage,
  noStoreHeaders,
  sessionToken,
} from "@/lib/server-api";

const SEGMENT_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/;

function isValidSegment(value: unknown): value is string {
  return typeof value === "string" &&
    SEGMENT_PATTERN.test(value) &&
    !value.endsWith(".");
}

function apiPath(owner: string, name: string, number?: number) {
  const base = "/api/v1/repositories/" +
    encodeURIComponent(owner) +
    "/" +
    encodeURIComponent(name) +
    "/issues";
  return number === undefined ? base : base + "/" + number;
}

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  const url = new URL(request.url);
  const owner = url.searchParams.get("owner");
  const name = url.searchParams.get("name");
  const numberRaw = url.searchParams.get("number");
  if (!isValidSegment(owner) || !isValidSegment(name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }

  const number = numberRaw === null ? undefined : Number(numberRaw);
  if (number !== undefined && (!Number.isSafeInteger(number) || number < 1)) {
    return NextResponse.json({ error: "Issue number must be a positive integer." }, { status: 400 });
  }

  try {
    const endpoint = number === undefined
      ? apiPath(owner, name) + (url.searchParams.get("state") ? "?state=" + encodeURIComponent(url.searchParams.get("state")!) : "")
      : apiPath(owner, name, number) + "/comments";
    const upstream = await fetch(apiBaseUrl() + endpoint, {
      headers: noStoreHeaders(token),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok ? payload : { error: apiErrorMessage(payload, "Issues could not be loaded.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  let input: { owner?: unknown; name?: unknown; number?: unknown; title?: unknown; body?: unknown };
  try {
    input = await request.json();
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }

  if (!isValidSegment(input.owner) || !isValidSegment(input.name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  const isComment = input.number !== undefined;
  const number = isComment ? Number(input.number) : undefined;
  if (isComment && (!Number.isSafeInteger(number) || (number ?? 0) < 1)) {
    return NextResponse.json({ error: "Issue number must be a positive integer." }, { status: 400 });
  }
  if (typeof input.body !== "string" || input.body.trim().length === 0) {
    return NextResponse.json({ error: "A non-empty body is required." }, { status: 400 });
  }

  let endpoint = apiPath(input.owner, input.name);
  let payload: Record<string, string>;
  if (isComment) {
    endpoint += "/" + number + "/comments";
    payload = { body: input.body };
  } else {
    if (typeof input.title !== "string" || input.title.trim().length === 0 || input.title.length > 200) {
      return NextResponse.json({ error: "Issue title must contain 1-200 characters." }, { status: 400 });
    }
    payload = { title: input.title, body: input.body };
  }

  try {
    const upstream = await fetch(apiBaseUrl() + endpoint, {
      method: "POST",
      headers: { ...noStoreHeaders(token), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      cache: "no-store",
    });
    const result = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok ? result : { error: apiErrorMessage(result, "The issue action failed.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}

export async function PATCH(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  let input: { owner?: unknown; name?: unknown; number?: unknown; title?: unknown; body?: unknown; state?: unknown };
  try {
    input = await request.json();
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }

  if (!validateOwnerAndName(input.owner, input.name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  const number = Number(input.number);
  if (!Number.isSafeInteger(number) || number < 1) {
    return NextResponse.json({ error: "Issue number must be a positive integer." }, { status: 400 });
  }
  const payload: Record<string, string> = {};
  if (input.title !== undefined) {
    if (typeof input.title !== "string" || input.title.trim().length === 0 || input.title.length > 200) {
      return NextResponse.json({ error: "Issue title must contain 1-200 characters." }, { status: 400 });
    }
    payload.title = input.title;
  }
  if (input.body !== undefined) {
    if (typeof input.body !== "string" || input.body.length > 100_000) {
      return NextResponse.json({ error: "Issue body must be 100000 characters or fewer." }, { status: 400 });
    }
    payload.body = input.body;
  }
  if (input.state !== undefined) {
    if (input.state !== "open" && input.state !== "closed") {
      return NextResponse.json({ error: "Issue state must be open or closed." }, { status: 400 });
    }
    payload.state = input.state;
  }
  if (Object.keys(payload).length === 0) {
    return NextResponse.json({ error: "At least one issue field must be updated." }, { status: 400 });
  }

  try {
    const upstream = await fetch(apiBaseUrl() + apiPath(input.owner, input.name, number), {
      method: "PUT",
      headers: { ...noStoreHeaders(token), "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      cache: "no-store",
    });
    const result = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok ? result : { error: apiErrorMessage(result, "The issue could not be updated.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}
