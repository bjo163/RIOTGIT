"use client";

import Link from "next/link";
import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";

export default function LoginPage() {
  const router = useRouter();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setLoading(true);

    try {
      const response = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      const payload = (await response.json().catch(() => ({}))) as { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Sign-in failed. Try again.");
        return;
      }
      router.replace("/");
      router.refresh();
    } catch {
      setError("Could not reach the R!OTGIT web gateway.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="login-shell">
      <section className="login-card">
        <Link className="brand login-brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </Link>
        <div className="login-eyebrow"><span className="live-dot" /> PRIVATE NODE / IDENTITY CHECK</div>
        <h1>CLAIM YOUR<br /><em>TERRITORY.</em></h1>
        <p className="login-intro">Your code lives here. Sign in to enter your workspace.</p>
        <form className="login-form" onSubmit={submit}>
          <label htmlFor="username">USERNAME</label>
          <input
            id="username"
            name="username"
            autoComplete="username"
            required
            maxLength={100}
            value={username}
            onChange={(event) => setUsername(event.target.value)}
            placeholder="your-handle"
          />
          <label htmlFor="password">PASSWORD</label>
          <input
            id="password"
            name="password"
            type="password"
            autoComplete="current-password"
            required
            maxLength={1024}
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            placeholder="••••••••••••"
          />
          {error ? <p className="login-error" role="alert">{error}</p> : null}
          <button className="button button-lime login-submit" type="submit" disabled={loading}>
            {loading ? "CHECKING CREDENTIALS..." : "ENTER THE NETWORK"} <span>↗</span>
          </button>
        </form>
        <div className="login-foot"><span>R!OTGIT / AUTH GATE 001</span><Link href="/">BACK TO NODE ↗</Link></div>
      </section>
      <aside className="login-poster" aria-label="R!OTGIT brand statement">
        <span className="eyebrow">THE UNDERGROUND CODE NETWORK</span>
        <strong>BREAK<br /><em>THE RULES.</em></strong>
        <span className="poster-rule" />
        <p>OWN YOUR CODE.<br />OWN YOUR WORKFLOW.</p>
        <small>SELF HOSTED / LOCAL FIRST / BUILDER OWNED</small>
        <div className="poster-stamp">R!<span>OWN IT</span></div>
      </aside>
    </main>
  );
}
