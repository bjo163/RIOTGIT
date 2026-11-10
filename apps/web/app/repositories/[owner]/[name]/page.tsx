"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";

type TreeEntry = {
  name: string;
  path: string;
  mode: string;
  kind: string;
  object_id: string;
};

type TreeResult = {
  owner: string;
  repository: string;
  reference: string;
  path: string;
  entries: TreeEntry[];
};

type BlobResult = {
  path: string;
  size: number;
  binary: boolean;
  content: string | null;
};

type CommitEntry = {
  id: string;
  author_name: string;
  author_email: string;
  authored_at: number;
  subject: string;
};

type CommitsResult = {
  commits: CommitEntry[];
};

export default function RepositoryPage() {
  const params = useParams<{ owner: string; name: string }>();
  const owner = params.owner;
  const name = params.name;
  const [reference, setReference] = useState("HEAD");
  const [path, setPath] = useState("");
  const [tree, setTree] = useState<TreeResult | null>(null);
  const [blob, setBlob] = useState<BlobResult | null>(null);
  const [commits, setCommits] = useState<CommitEntry[]>([]);
  const [view, setView] = useState<"code" | "history">("code");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const loadTree = useCallback(async (nextPath: string, selectedRef: string) => {
    setLoading(true);
    setError("");
    setBlob(null);
    try {
      const query = new URLSearchParams({
        resource: "tree",
        owner,
        name,
        reference: selectedRef,
        path: nextPath,
      });
      const response = await fetch("/api/repository-data?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as TreeResult & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Could not load this repository path.");
        setTree(null);
        return;
      }
      setTree(payload);
      setPath(nextPath);
      setView("code");
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name]);

  const openFile = useCallback(async (filePath: string) => {
    setLoading(true);
    setError("");
    try {
      const query = new URLSearchParams({
        resource: "blob",
        owner,
        name,
        reference,
        path: filePath,
      });
      const response = await fetch("/api/repository-data?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as BlobResult & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Could not load file.");
        return;
      }
      setBlob(payload);
      setTree(null);
      setPath(filePath);
      setView("code");
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name, reference]);

  const loadHistory = useCallback(async () => {
    setView("history");
    setLoading(true);
    setError("");
    try {
      const query = new URLSearchParams({
        resource: "commits",
        owner,
        name,
        reference,
        limit: "50",
      });
      const response = await fetch("/api/repository-data?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as CommitsResult & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Could not load commit history.");
        return;
      }
      setCommits(payload.commits ?? []);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name, reference]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void loadTree("", "HEAD");
    }, 0);
    return () => window.clearTimeout(timer);
  }, [loadTree]);

  const crumbs = path ? path.split("/") : [];

  function openCrumb(index: number) {
    void loadTree(crumbs.slice(0, index + 1).join("/"), reference);
  }

  return (
    <main className="repo-browser">
      <header className="repo-browser-top">
        <Link className="brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </Link>
        <div className="repo-browser-breadcrumb">
          <Link href="/">WORKSPACE</Link><b>/</b><strong>{owner}/{name}</strong>
        </div>
      </header>

      <section className="repo-browser-head">
        <div>
          <div className="eyebrow muted">SOURCE / REPOSITORY</div>
          <h1>{owner}<span>/</span>{name}</h1>
          <p>Native Git repository · Ref {reference}</p>
        </div>
        <div className="repo-browser-actions">
          <button type="button" className="mini-action" onClick={() => void loadTree("", reference)}>↻ REFRESH</button>
          <Link href="/" className="text-link">← BACK TO DASHBOARD</Link>
        </div>
      </section>

      <nav className="repo-tabs" aria-label="Repository views">
        <button
          type="button"
          className={view === "code" ? "repo-tab active" : "repo-tab"}
          onClick={() => void loadTree("", reference)}
        >⑂ CODE</button>
        <button
          type="button"
          className={view === "history" ? "repo-tab active" : "repo-tab"}
          onClick={() => void loadHistory()}
        >◷ HISTORY</button>
        <span className="repo-ref-label">
          REF
          <input
            value={reference}
            onChange={(event) => setReference(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") void loadTree("", reference);
            }}
            aria-label="Git reference"
          />
          <button type="button" className="text-link" onClick={() => void loadTree("", reference)}>LOAD</button>
        </span>
      </nav>

      {error ? <p className="login-error dashboard-error" role="alert">{error}</p> : null}

      {view === "code" ? (
        <section className="repo-browser-panel">
          <div className="repo-pathbar">
            <button type="button" onClick={() => void loadTree("", reference)}>ROOT</button>
            {crumbs.map((crumb, index) => (
              <span key={crumb + index}><b>/</b><button type="button" onClick={() => openCrumb(index)}>{crumb}</button></span>
            ))}
          </div>
          {loading ? (
            <div className="repo-browser-empty">READING REPOSITORY OBJECTS...</div>
          ) : blob ? (
            <div className="blob-view">
              <div className="blob-toolbar">
                <strong>{blob.path}</strong>
                <span>{blob.size.toLocaleString()} BYTES</span>
                <button
                  type="button"
                  className="text-link"
                  onClick={() => void loadTree(path.split("/").slice(0, -1).join("/"), reference)}
                >BACK TO TREE</button>
              </div>
              {blob.binary || blob.content === null ? (
                <div className="repo-browser-empty">BINARY FILE / PREVIEW DISABLED</div>
              ) : (
                <pre className="source-code"><code>{blob.content}</code></pre>
              )}
            </div>
          ) : (
            <div className="repo-tree">
              {tree?.entries?.length ? tree.entries.map((entry) => (
                <button
                  type="button"
                  className="repo-tree-row"
                  key={entry.path}
                  onClick={() => entry.kind === "tree"
                    ? void loadTree(entry.path, reference)
                    : void openFile(entry.path)}
                >
                  <span className={entry.kind === "tree" ? "tree-icon folder" : "tree-icon file"}>
                    {entry.kind === "tree" ? "▰" : "▤"}
                  </span>
                  <strong>{entry.name}</strong>
                  <small>{entry.kind.toUpperCase()}</small>
                  <code>{entry.object_id.slice(0, 10)}</code>
                </button>
              )) : (
                <div className="repo-browser-empty">THIS DIRECTORY IS EMPTY.</div>
              )}
            </div>
          )}
        </section>
      ) : (
        <section className="repo-browser-panel">
          {loading ? (
            <div className="repo-browser-empty">READING COMMIT HISTORY...</div>
          ) : commits.length ? (
            <div className="commit-list">
              {commits.map((commit) => (
                <article className="commit-row" key={commit.id}>
                  <span className="commit-mark">↗</span>
                  <div>
                    <strong>{commit.subject}</strong>
                    <small>{commit.author_name} · {new Date(commit.authored_at * 1000).toLocaleString()}</small>
                  </div>
                  <code>{commit.id.slice(0, 12)}</code>
                </article>
              ))}
            </div>
          ) : (
            <div className="repo-browser-empty">NO COMMITS FOUND FOR THIS REF.</div>
          )}
        </section>
      )}
    </main>
  );
}
