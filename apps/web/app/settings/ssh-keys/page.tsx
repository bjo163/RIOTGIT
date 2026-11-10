"use client";

import Link from "next/link";
import { useCallback, useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";

type SshKey = {
  id: string;
  fingerprint: string;
  key_type: string;
  comment: string;
  created_at: string;
};

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
}

export default function SshKeysPage() {
  const router = useRouter();
  const [keys, setKeys] = useState<SshKey[]>([]);
  const [publicKey, setPublicKey] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [removingId, setRemovingId] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const loadKeys = useCallback(async (signal?: AbortSignal) => {
    try {
      const response = await fetch("/api/user/ssh-keys", {
        cache: "no-store",
        signal,
      });
      const payload = (await response.json().catch(() => ({}))) as SshKey[] | { error?: string };
      if (response.status === 401) {
        router.replace("/login");
        return;
      }
      if (!response.ok || !Array.isArray(payload)) {
        setError(!Array.isArray(payload) ? payload.error ?? "SSH keys could not be loaded." : "SSH keys could not be loaded.");
        return;
      }
      setKeys(payload);
    } catch (cause) {
      if (cause instanceof Error && cause.name === "AbortError") return;
      setError("Could not reach the R!OTGIT node.");
    } finally {
      if (!signal?.aborted) setLoading(false);
    }
  }, [router]);

  useEffect(() => {
    const controller = new AbortController();
    void loadKeys(controller.signal);
    return () => controller.abort();
  }, [loadKeys]);

  async function addKey(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError("");
    setNotice("");
    try {
      const response = await fetch("/api/user/ssh-keys", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ public_key: publicKey.trim() }),
      });
      const payload = (await response.json().catch(() => ({}))) as SshKey & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "SSH key could not be added.");
        return;
      }
      setKeys((current) => [payload, ...current.filter((item) => item.id !== payload.id)]);
      setPublicKey("");
      setNotice("PUBLIC KEY REGISTERED. IT CAN NOW AUTHENTICATE TO THE SSH GATEWAY.");
    } catch {
      setError("Could not reach the R!OTGIT node.");
    } finally {
      setSaving(false);
    }
  }

  async function revokeKey(key: SshKey) {
    const confirmed = window.confirm(
      "Revoke " + key.fingerprint + "? New SSH authentication using this key will be denied.",
    );
    if (!confirmed) return;

    setRemovingId(key.id);
    setError("");
    setNotice("");
    try {
      const query = new URLSearchParams({ id: key.id });
      const response = await fetch("/api/user/ssh-keys?" + query.toString(), { method: "DELETE" });
      if (!response.ok) {
        const payload = (await response.json().catch(() => ({}))) as { error?: string };
        setError(payload.error ?? "SSH key could not be revoked.");
        return;
      }
      setKeys((current) => current.filter((item) => item.id !== key.id));
      setNotice("SSH KEY REVOKED.");
    } catch {
      setError("Could not reach the R!OTGIT node.");
    } finally {
      setRemovingId("");
    }
  }

  return (
    <main className="ssh-settings-shell">
      <header className="ssh-settings-topbar">
        <Link className="brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </Link>
        <div className="breadcrumb"><span>ACCOUNT</span><b>/</b><strong>SSH KEYS</strong></div>
        <Link href="/" className="text-link">← BACK TO WORKSPACE</Link>
      </header>

      <div className="ssh-settings-content">
        <div className="eyebrow muted">IDENTITY / TRANSPORT CREDENTIALS</div>
        <h1>YOUR KEYS.<br /><em>YOUR GATE.</em></h1>
        <p className="ssh-settings-intro">
          Register public keys to authenticate to your R!OTGIT SSH gateway. Private keys never leave your device.
        </p>

        {error ? <p className="login-error" role="alert">{error}</p> : null}
        {notice ? <p className="ssh-settings-notice" role="status">{notice}</p> : null}

        <section className="ssh-key-form panel">
          <div className="panel-heading">
            <div><span className="eyebrow muted">REGISTER / 01</span><h2>ADD PUBLIC KEY</h2></div>
            <span className="ssh-key-form-mark">R!</span>
          </div>
          <form onSubmit={addKey}>
            <label className="ssh-field-label" htmlFor="ssh-public-key">OPENSSH PUBLIC KEY</label>
            <input
              id="ssh-public-key"
              className="ssh-public-key-input"
              value={publicKey}
              onChange={(event) => setPublicKey(event.target.value)}
              placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA..."
              autoComplete="off"
              autoCapitalize="none"
              spellCheck={false}
              maxLength={16_384}
              required
            />
            <p className="ssh-field-hint">Accepted formats: Ed25519, NIST P-256 ECDSA, security-key, and RSA public keys.</p>
            <div className="ssh-form-actions">
              <button className="button button-lime" type="submit" disabled={saving || !publicKey.trim()}>
                {saving ? "REGISTERING..." : "REGISTER PUBLIC KEY"} <span>↗</span>
              </button>
              <span className="ssh-form-meta">PUBLIC KEY ONLY / NEVER UPLOAD A PRIVATE KEY</span>
            </div>
          </form>
        </section>

        <section className="ssh-key-list panel">
          <div className="panel-heading">
            <div><span className="eyebrow muted">TRUSTED DEVICES / 02</span><h2>REGISTERED KEYS</h2></div>
            <button type="button" className="text-link" onClick={() => { setLoading(true); setError(""); void loadKeys(); }}>↻ REFRESH</button>
          </div>
          {loading ? (
            <div className="ssh-empty-state">READING YOUR KEY REGISTRY...</div>
          ) : keys.length === 0 ? (
            <div className="ssh-empty-state">
              <strong>NO KEYS REGISTERED.</strong>
              <span>Add a public key to enable key-based authentication.</span>
            </div>
          ) : (
            <div className="ssh-key-rows">
              {keys.map((key) => (
                <article className="ssh-key-row" key={key.id}>
                  <div className="ssh-key-mark">⌘</div>
                  <div className="ssh-key-details">
                    <strong>{key.comment || "UNTITLED KEY"}</strong>
                    <code>{key.fingerprint}</code>
                    <small>{key.key_type} / ADDED {formatDate(key.created_at)}</small>
                  </div>
                  <button
                    type="button"
                    className="text-link ssh-revoke-button"
                    onClick={() => void revokeKey(key)}
                    disabled={removingId === key.id}
                  >
                    {removingId === key.id ? "REVOKING..." : "REVOKE"}
                  </button>
                </article>
              ))}
            </div>
          )}
        </section>

        <aside className="ssh-transport-note">
          <strong>TRANSPORT STATUS / CHECK BEFORE CONNECTING</strong>
          <p>
            SSH public-key registration is available now. SSH clone/push requires the optional OpenSSH gateway to be running
            with the <code>ssh</code> Compose profile. The gateway is still under integration verification.
          </p>
          <code>git clone ssh://git@HOST:2222/OWNER/REPOSITORY.git</code>
        </aside>
      </div>
    </main>
  );
}
