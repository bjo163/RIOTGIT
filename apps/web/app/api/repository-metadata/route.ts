import { NextResponse } from "next/server";
import {
  apiBaseUrl,
  apiErrorMessage,
  noStoreHeaders,
  sessionToken,
} from "@/lib/server-api";

const SEGMENT_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/;

function isSegment(value: unknown): value is string {
  return typeof value === "string" && SEGMENT_PATTERN.test(value) && !value.endsWith(".");
}

function issueNumber(value: unknown): number | null {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : null;
}

function repositoryPath(owner: string, name: string) {
  return "/api/v1/repositories/" + encodeURIComponent(owner) + "/" + encodeURIComponent(name);
}

async function forward(
  path: string,
  token: string,
  method = "GET",
  payload?: Record<string, unknown>,
) {
  const upstream = await fetch(apiBaseUrl() + path, {
    method,
    headers: {
      ...noStoreHeaders(token),
      ...(payload ? { "Content-Type": "application/json" } : {}),
    },
    ...(payload ? { body: JSON.stringify(payload) } : {}),
    cache: "no-store",
  });
  const result = await upstream.json().catch(() => null);
  return NextResponse.json(
    upstream.ok ? result : { error: apiErrorMessage(result, "Repository metadata action failed.") },
    { status: upstream.status },
  );
}

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  const url = new URL(request.url);
  const owner = url.searchParams.get("owner");
  const name = url.searchParams.get("name");
  const resource = url.searchParams.get("resource");
  if (!isSegment(owner) || !isSegment(name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }

  const base = repositoryPath(owner, name);
  if (resource === "labels") return forward(base + "/labels", token);
  if (resource === "milestones") return forward(base + "/milestones", token);
  if (resource === "issue-labels") {
    const number = issueNumber(url.searchParams.get("number"));
    if (!number) return NextResponse.json({ error: "A positive issue number is required." }, { status: 400 });
    return forward(base + "/issues/" + number + "/labels", token);
  }
  return NextResponse.json({ error: "Choose labels, milestones, or issue-labels." }, { status: 400 });
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
  if (!isSegment(input.owner) || !isSegment(input.name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  const base = repositoryPath(input.owner, input.name);
  if (input.resource === "labels") {
    if (typeof input.name !== "string" || input.name.trim().length === 0 || input.name.length > 50) {
      return NextResponse.json({ error: "Label name must contain 1-50 characters." }, { status: 400 });
    }
    return forward(base + "/labels", token, "POST", {
      name: input.label_name,
      color: typeof input.color === "string" ? input.color : "D7FF38",
      description: typeof input.description === "string" ? input.description : undefined,
    });
  }
  if (input.resource === "milestones") {
    if (typeof input.title !== "string" || input.title.trim().length === 0 || input.title.length > 100) {
      return NextResponse.json({ error: "Milestone title must contain 1-100 characters." }, { status: 400 });
    }
    return forward(base + "/milestones", token, "POST", {
      title: input.title,
      description: typeof input.description === "string" ? input.description : undefined,
      due_on: typeof input.due_on === "string" && input.due_on ? input.due_on : undefined,
    });
  }
  return NextResponse.json({ error: "Choose labels or milestones." }, { status: 400 });
}

export async function PUT(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });
  let input: Record<string, unknown>;
  try {
    input = await request.json();
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }
  if (!isSegment(input.owner) || !isSegment(input.name)) {
    return NextResponse.json({ error: "A valid owner and repository name are required." }, { status: 400 });
  }
  const number = issueNumber(input.number);
  if (!number) return NextResponse.json({ error: "A positive issue number is required." }, { status: 400 });
  const base = repositoryPath(input.owner, input.name);
  if (input.resource === "issue-label") {
    if (typeof input.label_id !== "string" || input.label_id.length > 128) {
      return NextResponse.json({ error: "A valid label ID is required." }, { status: 400 });
    }
    return forward(base + "/issues/" + number + "/labels/" + encodeURIComponent(input.label_id), token, "PUT");
  }
  if (input.resource === "issue-milestone") {
    const milestoneNumber = issueNumber(input.milestone_number);
    if (!milestoneNumber) return NextResponse.json({ error: "A positive milestone number is required." }, { status: 400 });
    return forward(base + "/issues/" + number + "/milestone", token, "PUT", { number: milestoneNumber });
  }
  return NextResponse.json({ error: "Choose issue-label or issue-milestone." }, { status: 400 });
}

export async function DELETE(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });
  const url = new URL(request.url);
  const owner = url.searchParams.get("owner");
  const name = url.searchParams.get("name");
  const resource = url.searchParams.get("resource");
  const number = issueNumber(url.searchParams.get("number"));
  if (!isSegment(owner) || !isSegment(name) || !number) {
    return NextResponse.json({ error: "A valid repository and issue number are required." }, { status: 400 });
  }
  const base = repositoryPath(owner, name);
  if (resource === "issue-label") {
    const labelId = url.searchParams.get("label_id");
    if (!labelId || labelId.length > 128) {
      return NextResponse.json({ error: "A valid label ID is required." }, { status: 400 });
    }
    return forward(base + "/issues/" + number + "/labels/" + encodeURIComponent(labelId), token, "DELETE");
  }
  if (resource === "issue-milestone") {
    return forward(base + "/issues/" + number + "/milestone", token, "DELETE");
  }
  return NextResponse.json({ error: "Choose issue-label or issue-milestone." }, { status: 400 });
}
