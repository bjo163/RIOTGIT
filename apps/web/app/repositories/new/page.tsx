"use client";

import Link from "next/link";
import { FormEvent, useState } from "react";
import { useRouter } from "next/navigation";

export default function NewRepositoryPage() {
  const router = useRouter();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [visibility, setVisibility] = useState("private");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setLoading(true);
    try {
      const response = await fetch("/api/repositories", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, description, visibility }),
      });
      const payload = (await response.json().catch(() => ({}))) as { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Repository creation failed.");
        return;
      }
      router.replace("/");
      router.refresh();
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }

  return (
    <main className="content page-content">
      <header className="new-repo-top">
        <Link className="brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </Link>
        <Link className="text-link" href="/">← BACK TO TERRITORY</Link>
      </header>
      <section className="new-repo-panel">
        <div className="eyebrow muted">SOURCE / REPOSITORY INITIALIZER</div>
        <h1>MAKE SOME<br /><em>NOISE.</em></h1>
        <p>Create a native bare Git repository in your R!OTGIT node.</p>
        <form className="login-form" onSubmit={submit}>
          <label htmlFor="repo-name">REPOSITORY NAME</label>
          <input
            id="repo-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            minLength={1}
            maxLength={100}
            pattern="[A-Za-z0-9][A-Za-z0-9._-]*"
            placeholder="my-next-build"
          />
          <label htmlFor="repo-description">DESCRIPTION <span className="field-optional">OPTIONAL</span></label>
          <input
            id="repo-description"
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            maxLength={1000}
            placeholder="What are you building?"
          />
          <label htmlFor="repo-visibility">VISIBILITY</label>
          <select id="repo-visibility" value={visibility} onChange={(event) => setVisibility(event.target.value)}>
            <option value="private">Private — only you and granted collaborators</option>
            <option value="internal">Internal — authenticated users</option>
            <option value="public">Public — anyone can read</option>
          </select>
          {error ? <p className="login-error" role="alert">{error}</p> : null}
          <button className="button button-lime login-submit" type="submit" disabled={loading}>
            {loading ? "INITIALIZING BARE REPOSITORY..." : "CREATE REPOSITORY"} <span>↗</span>
          </button>
        </form>
      </section>
    </main>
  );
}
