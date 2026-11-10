import { NextResponse } from "next/server";
import {
  apiBaseUrl,
  apiErrorMessage,
  noStoreHeaders,
  sessionToken,
} from "@/lib/server-api";

type SshKeyPayload = {
  public_key?: unknown;
};

function errorResponse(payload: unknown, fallback: string, status: number) {
  return NextResponse.json({ error: apiErrorMessage(payload, fallback) }, { status });
}

export async function GET(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  try {
    const upstream = await fetch(apiBaseUrl() + "/api/v1/user/ssh-keys", {
      headers: noStoreHeaders(token),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    if (!upstream.ok) {
      return errorResponse(payload, "SSH keys could not be loaded.", upstream.status);
    }
    return NextResponse.json(payload, {
      headers: { "Cache-Control": "no-store, private" },
    });
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}

export async function POST(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  let input: SshKeyPayload;
  try {
    input = (await request.json()) as SshKeyPayload;
  } catch {
    return NextResponse.json({ error: "A valid JSON body is required." }, { status: 400 });
  }

  if (
    typeof input.public_key !== "string" ||
    input.public_key.length === 0 ||
    input.public_key.length > 16_384 ||
    input.public_key.includes("\n") ||
    input.public_key.includes("\r")
  ) {
    return NextResponse.json(
      { error: "Enter one OpenSSH public key line (maximum 16,384 characters)." },
      { status: 400 },
    );
  }

  try {
    const upstream = await fetch(apiBaseUrl() + "/api/v1/user/ssh-keys", {
      method: "POST",
      headers: {
        ...noStoreHeaders(token),
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ public_key: input.public_key.trim() }),
      cache: "no-store",
    });
    const payload = await upstream.json().catch(() => null);
    if (!upstream.ok) {
      return errorResponse(payload, "SSH public key could not be registered.", upstream.status);
    }
    return NextResponse.json(payload, {
      status: upstream.status,
      headers: { "Cache-Control": "no-store, private" },
    });
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}

export async function DELETE(request: Request) {
  const token = sessionToken(request);
  if (!token) return NextResponse.json({ error: "Sign in to continue." }, { status: 401 });

  const url = new URL(request.url);
  const keyId = url.searchParams.get("id");
  if (!keyId || !/^[a-fA-F0-9]{32}$/.test(keyId)) {
    return NextResponse.json({ error: "A valid SSH key ID is required." }, { status: 400 });
  }

  try {
    const upstream = await fetch(
      apiBaseUrl() + "/api/v1/user/ssh-keys/" + encodeURIComponent(keyId),
      {
        method: "DELETE",
        headers: noStoreHeaders(token),
        cache: "no-store",
      },
    );
    if (upstream.status === 204) return new NextResponse(null, { status: 204 });
    const payload = await upstream.json().catch(() => null);
    if (!upstream.ok) {
      return errorResponse(payload, "SSH public key could not be revoked.", upstream.status);
    }
    return NextResponse.json(payload ?? { success: true }, {
      status: upstream.status,
      headers: { "Cache-Control": "no-store, private" },
    });
  } catch {
    return NextResponse.json({ error: "R!OTGIT API is unavailable." }, { status: 502 });
  }
}
