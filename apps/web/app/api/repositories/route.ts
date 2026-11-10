import { NextResponse } from "next/server";
import { apiBaseUrl, apiErrorMessage, noStoreHeaders, sessionToken } from "@/lib/server-api";

type CurrentUser = { username?: unknown };
type CreateRepositoryPayload = { name?: unknown; description?: unknown; visibility?: unknown };

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  try {
    const upstream = await fetch(apiBaseUrl() + "/api/v1/repositories", {
      headers: noStoreHeaders(token),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok ? payload : { error: apiErrorMessage(payload, "Could not load repositories.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  let input: CreateRepositoryPayload;
  try {
    input = (await request.json()) as CreateRepositoryPayload;
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }

  if (
    typeof input.name !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/.test(input.name) ||
    input.name.endsWith(".")
  ) {
    return NextResponse.json({ error: "Repository name must start with a letter or digit and use only letters, digits, dot, dash, or underscore." }, { status: 400 });
  }
  if (input.description !== undefined && typeof input.description !== "string") {
    return NextResponse.json({ error: "Description must be text." }, { status: 400 });
  }
  if (typeof input.description === "string" && input.description.length > 1000) {
    return NextResponse.json({ error: "Description must be 1000 characters or fewer." }, { status: 400 });
  }
  const visibility = input.visibility ?? "private";
  if (!["private", "public", "internal"].includes(String(visibility))) {
    return NextResponse.json({ error: "Choose private, public, or internal visibility." }, { status: 400 });
  }

  try {
    const userResponse = await fetch(apiBaseUrl() + "/api/v1/auth/me", {
      headers: noStoreHeaders(token),
      cache: "no-store",
    });
    const user = (await userResponse.json().catch(() => null)) as CurrentUser | null;
    if (!userResponse.ok || !user || typeof user.username !== "string") {
      return NextResponse.json({ error: "Your session is no longer valid." }, { status: 401 });
    }

    const upstream = await fetch(apiBaseUrl() + "/api/v1/repositories", {
      method: "POST",
      headers: { ...noStoreHeaders(token), "Content-Type": "application/json" },
      body: JSON.stringify({
        owner: user.username,
        name: input.name,
        description: input.description ?? null,
        visibility,
      }),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    return NextResponse.json(
      upstream.ok ? payload : { error: apiErrorMessage(payload, "Repository creation failed.") },
      { status: upstream.status },
    );
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}
