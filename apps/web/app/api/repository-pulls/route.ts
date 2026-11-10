import { NextResponse } from "next/server";
import {
  apiBaseUrl,
  apiErrorMessage,
  noStoreHeaders,
  sessionToken,
} from "@/lib/server-api";

const SEGMENT_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/;
const BRANCH_PATTERN = /^[A-Za-z0-9/._-]{1,200}$/;

function isSegment(value: unknown): value is string {
  return typeof value === "string" && SEGMENT_PATTERN.test(value) && !value.endsWith(".");
}

function isBranchName(value: unknown): value is string {
  return typeof value === "string" &&
    BRANCH_PATTERN.test(value) &&
    !value.startsWith("-") &&
    !value.endsWith(".") &&
    !value.endsWith("/") &&
    !value.includes("..") &&
    !value.includes("//") &&
    !value.includes("@{") &&
    value.split("/").every((part) => part.length > 0 && !part.startsWith(".") && !part.endsWith(".lock"));
}

function positiveNumber(value: unknown): number | null {
  const number = Number(value);
  return Number.isSafeInteger(number) && number > 0 ? number : null;
}

function repositoryPath(owner: string, name: string) {
  return "/api/v1/repositories/" + encodeURIComponent(owner) + "/" + encodeURIComponent(name);
}

async function forward(path: string, token: string, method = "GET", payload?: Record<string, unknown>) {
  const upstream = await fetch(apiBaseUrl() + path, {
    method,
    headers: {
      ...noStoreHeaders(token),
      ...(payload ? { "Content-Type": "application/json" } : {}),
    },
    ...(payload ? { body: JSON.stringify(payload) } : {}),
    cache: "no-store",
  });
  if (upstream.status === 204) return new NextResponse(null, { status: 204 });
  const result = await upstream.json().catch(() => null);
  return NextResponse.json(
    upstream.ok ? result : { error: apiErrorMessage(result, "Pull request action failed.") },
    { status: upstream.status },
  );
}

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });
  const url = new URL(request.url);
  const owner = url.searchParams.get("owner");
  const name = url.searchParams.get("name");
  const resource = url.searchParams.get("resource") ?? "list";
  if (!isSegment(owner) || !isSegment(name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  const base = repositoryPath(owner, name) + "/pulls";
  const state = url.searchParams.get("state");
  if (resource === "list") {
    const query = new URLSearchParams();
    if (state) query.set("state", state);
    const suffix = query.size ? "?" + query.toString() : "";
    return forward(base + suffix, token);
  }
  const number = positiveNumber(url.searchParams.get("number"));
  if (!number) return NextResponse.json({ error: "A positive pull request number is required." }, { status: 400 });
  if (resource === "detail") return forward(base + "/" + number, token);
  if (resource === "reviews") return forward(base + "/" + number + "/reviews", token);
  return NextResponse.json({ error: "Choose list, detail, or reviews." }, { status: 400 });
}

export async function POST(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });
  let input: Record<string, unknown>;
  try {
    input = await request.json();
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }
  if (!isSegment(input.owner) || !isSegment(input.name) || typeof input.resource !== "string") {
    return NextResponse.json({ error: "A valid repository and action are required." }, { status: 400 });
  }

  const base = repositoryPath(input.owner, input.name) + "/pulls";
  if (input.resource === "create") {
    if (typeof input.title !== "string" || input.title.trim().length === 0 || input.title.length > 200) {
      return NextResponse.json({ error: "Pull request title must contain 1-200 characters." }, { status: 400 });
    }
    if (!isBranchName(input.head_ref) || !isBranchName(input.base_ref) || input.head_ref === input.base_ref) {
      return NextResponse.json({ error: "Choose two different valid branch names." }, { status: 400 });
    }
    if (input.body !== undefined && (typeof input.body !== "string" || input.body.length > 100000)) {
      return NextResponse.json({ error: "Pull request description must be 100000 characters or fewer." }, { status: 400 });
    }
    return forward(base, token, "POST", {
      title: input.title,
      body: typeof input.body === "string" ? input.body : undefined,
      head_ref: input.head_ref,
      base_ref: input.base_ref,
    });
  }

  const number = positiveNumber(input.number);
  if (!number) return NextResponse.json({ error: "A positive pull request number is required." }, { status: 400 });
  if (input.resource === "review") {
    if (!["comment", "approved", "changes_requested"].includes(String(input.state))) {
      return NextResponse.json({ error: "Choose comment, approved, or changes_requested." }, { status: 400 });
    }
    if (input.body !== undefined && (typeof input.body !== "string" || input.body.length > 20000)) {
      return NextResponse.json({ error: "Review body must be 20000 characters or fewer." }, { status: 400 });
    }
    return forward(base + "/" + number + "/reviews", token, "POST", {
      state: input.state,
      body: typeof input.body === "string" ? input.body : undefined,
    });
  }
  if (input.resource === "merge") {
    return forward(base + "/" + number + "/merge", token, "POST");
  }
  return NextResponse.json({ error: "Choose create, review, or merge." }, { status: 400 });
}

export async function PATCH(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });
  let input: Record<string, unknown>;
  try {
    input = await request.json();
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }
  if (!isSegment(input.owner) || !isSegment(input.name)) {
    return NextResponse.json({ error: "A valid repository is required." }, { status: 400 });
  }
  const number = positiveNumber(input.number);
  if (!number || (input.state !== "open" && input.state !== "closed")) {
    return NextResponse.json({ error: "A pull request number and valid open/closed state are required." }, { status: 400 });
  }
  return forward(repositoryPath(input.owner, input.name) + "/pulls/" + number, token, "PUT", { state: input.state });
}
