"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";

type User = {
  id: string;
  username: string;
  display_name: string;
  role: string;
};

type Repository = {
  id: string;
  owner: string;
  name: string;
  description: string | null;
  visibility: "public" | "private" | "internal";
  state: string;
  created_at: string;
  updated_at: string;
};

function formatDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "—";
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium" }).format(date);
}

export default function Home() {
  const router = useRouter();
  const [user, setUser] = useState<User | null>(null);
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [signingOut, setSigningOut] = useState(false);

  useEffect(() => {
    const controller = new AbortController();

    async function loadWorkspace() {
      try {
        const meResponse = await fetch("/api/auth/me", {
          cache: "no-store",
          signal: controller.signal,
        });
        if (!meResponse.ok) {
          setUser(null);
          return;
        }

        const currentUser = (await meResponse.json()) as User;
        setUser(currentUser);

        const repoResponse = await fetch("/api/repositories", {
          cache: "no-store",
          signal: controller.signal,
        });
        if (!repoResponse.ok) {
          const payload = (await repoResponse.json().catch(() => ({}))) as { error?: string };
          setError(payload.error ?? "Could not load repositories.");
          return;
        }
        setRepositories((await repoResponse.json()) as Repository[]);
      } catch (cause) {
        if (cause instanceof Error && cause.name === "AbortError") return;
        setError("Could not connect to the R!OTGIT node.");
      } finally {
        if (!controller.signal.aborted) setLoading(false);
      }
    }

    void loadWorkspace();
    return () => controller.abort();
  }, []);

  async function logout() {
    setSigningOut(true);
    try {
      await fetch("/api/auth/logout", { method: "POST" });
      router.push("/login");
    } catch {
      setError("Sign-out could not reach the API. Try again.");
      setSigningOut(false);
    }
  }

  if (loading) {
    return (
      <main className="login-shell">
        <section className="login-card">
          <div className="brand">
            <span className="brand-mark">R!</span>
            <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
          </div>
          <div className="login-eyebrow"><span className="live-dot" /> CONNECTING TO YOUR NODE</div>
          <h1>CHECKING<br /><em>THE GATE.</em></h1>
        </section>
      </main>
    );
  }

  if (!user) {
    return (
      <main className="login-shell">
        <section className="login-card">
          <div className="brand">
            <span className="brand-mark">R!</span>
            <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
          </div>
          <div className="login-eyebrow"><span className="live-dot" /> PRIVATE NODE / SESSION REQUIRED</div>
          <h1>CLAIM YOUR<br /><em>TERRITORY.</em></h1>
          <p className="login-intro">Sign in to your self-hosted code network.</p>
          {error ? <p className="login-error" role="alert">{error}</p> : null}
          <Link href="/login" className="button button-lime login-submit">
            ENTER THE NETWORK <span>↗</span>
          </Link>
        </section>
        <aside className="login-poster">
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

  return (
    <main className="shell">
      <aside className="rail">
        <Link className="brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </Link>
        <div className="rail-caption">THE NETWORK</div>
        <nav className="nav" aria-label="Main navigation">
          <a className="nav-item active" href="#overview"><span>▦</span> Overview</a>
          <a className="nav-item" href="#repositories"><span>⌘</span> Repositories</a>
          <a className="nav-item" href="#activity"><span>↯</span> Activity</a>
          <a className="nav-item" href="#network"><span>◉</span> Network</a>
        </nav>
        <div className="rail-caption rail-bottom-caption">WORKSPACE</div>
        <div className="workspace-chip">
          <span className="workspace-avatar">{user.username.slice(0, 1).toUpperCase()}</span>
          <span><b>{user.display_name.toUpperCase()}</b><small>{user.role.toUpperCase()} / LOCAL NODE</small></span>
          <span className="chevron">⌄</span>
        </div>
        <div className="rail-footer"><span className="live-dot" /> SESSION ACTIVE <span className="mono">01</span></div>
      </aside>

      <section className="main-area" id="overview">
        <header className="topbar">
          <div className="breadcrumb"><span>WORKSPACE</span><b>/</b><strong>OVERVIEW</strong></div>
          <div className="top-actions">
            <Link className="search-button" href="/settings/ssh-keys"><span>⌘</span> SSH keys</Link>
            <Link className="search-button" href="/repositories/new"><span>＋</span> New repository</Link>
            <button
              className="icon-button signout-button"
              type="button"
              onClick={logout}
              disabled={signingOut}
              aria-label="Sign out"
              title="Sign out"
            >↗</button>
            <div className="user-avatar" aria-label={"Signed in as " + user.username}>
              {user.username.slice(0, 1).toUpperCase()}
            </div>
          </div>
        </header>

        <div className="content">
          <section className="hero">
            <div className="hero-grain" />
            <div className="hero-copy">
              <div className="eyebrow"><span className="live-dot" /> SESSION VERIFIED / {user.username.toUpperCase()}</div>
              <h1>BREAK THE<br /><em>RULES.</em></h1>
              <p>Own your code. Build your network.<br />No permission required.</p>
              <div className="hero-actions">
                <Link href="/repositories/new" className="button button-lime">CREATE REPOSITORY <span>↗</span></Link>
                <a href="#network" className="text-link">NODE STATUS <span>→</span></a>
              </div>
            </div>
            <div className="hero-stamp" aria-hidden="true">
              <div className="stamp-ring">SELF HOSTED · LOCAL FIRST · OPEN NETWORK ·</div>
              <div className="stamp-core">R!<small>OWN IT</small></div>
            </div>
            <div className="hero-index">ISSUE 001 <span>—</span> CODE WITHOUT PERMISSION</div>
          </section>

          {error ? <p className="login-error dashboard-error" role="alert">{error}</p> : null}

          <div className="section-heading">
            <div><div className="eyebrow muted">CONTROL ROOM / 01</div><h2>YOUR TERRITORY<span>_</span></h2></div>
            <Link className="text-link" href="/repositories/new">NEW REPOSITORY ↗</Link>
          </div>

          <section className="stat-grid" aria-label="Workspace summary">
            <article className="stat-card">
              <span>VISIBLE REPOSITORIES</span>
              <strong>{String(repositories.length).padStart(2, "0")}</strong>
              <small>FILTERED BY YOUR ACCESS</small>
            </article>
            <article className="stat-card">
              <span>COLLABORATION</span>
              <strong>—</strong>
              <small>PR / REVIEW MODULE IN PROGRESS</small>
            </article>
            <article className="stat-card">
              <span>IDENTITY</span>
              <strong className="stat-online">ONLINE<span className="live-dot" /></strong>
              <small>{user.role.toUpperCase()} SESSION</small>
            </article>
            <article className="stat-card stat-card-accent">
              <span>NETWORK MODE</span>
              <strong>LOCAL</strong>
              <small>P2P ADAPTER DISABLED</small>
            </article>
          </section>

          <div className="work-grid">
            <section id="repositories" className="panel repo-panel">
              <div className="panel-heading">
                <div><span className="eyebrow muted">SOURCE / 02</span><h3>YOUR REPOSITORIES</h3></div>
                <Link href="/repositories/new" className="mini-action">＋ NEW REPO</Link>
              </div>
              {repositories.length > 0 ? (
                <div className="repo-list">
                  {repositories.map((repo) => (
                    <article className="repo-row" key={repo.id}>
                      <div className="repo-icon">⑂</div>
                      <div className="repo-info">
                        <strong><Link href={"/repositories/" + encodeURIComponent(repo.owner) + "/" + encodeURIComponent(repo.name)}>{repo.owner}/{repo.name}</Link></strong>
                        <span>{repo.description || "No description yet"}</span>
                      </div>
                      <div className="repo-language">{repo.visibility.toUpperCase()}</div>
                      <div className="repo-branch">{repo.state.toUpperCase()}</div>
                      <div className={"repo-state " + (repo.state === "ready" ? "clean" : "review")}>
                        {repo.state === "ready" ? "READY" : "PENDING"}
                      </div>
                      <div className="repo-updated">{formatDate(repo.updated_at)}</div>
                      <span className="row-more" aria-hidden="true">↗</span>
                    </article>
                  ))}
                </div>
              ) : (
                <div className="empty-repositories">
                  <strong>NO REPOSITORIES. NO PROBLEM.</strong>
                  <span>Your territory is empty. Create the first native Git repository.</span>
                  <Link href="/repositories/new" className="text-link">CREATE YOUR FIRST REPO ↗</Link>
                </div>
              )}
            </section>

            <section id="activity" className="panel activity-panel">
              <div className="panel-heading">
                <div><span className="eyebrow muted">SIGNAL / 03</span><h3>EVENT FEED</h3></div>
                <span className="feed-live"><i /> WAITING</span>
              </div>
              <div className="event-empty">
                <span className="event-empty-mark">↯</span>
                <strong>NO SIGNALS YET.</strong>
                <span>Pushes, reviews, releases, and network events will show here when their services are enabled.</span>
              </div>
              <span className="panel-link">EVENT PIPELINE / IN DEVELOPMENT</span>
            </section>
          </div>

          <section id="network" className="network-strip">
            <div className="network-symbol">⌁</div>
            <div className="network-copy">
              <span className="eyebrow">NETWORK STATUS / EXPERIMENTAL LATER</span>
              <strong>ONE NODE. FULL CONTROL.</strong>
              <small>Mirrors, federation, and peer sync will arrive as optional adapters.</small>
            </div>
            <span className="network-state"><i /> SINGLE NODE</span>
          </section>

          <footer className="footer">
            <span>R!OTGIT © 2026</span>
            <span>THE UNDERGROUND CODE NETWORK</span>
            <span>BUILT FOR THE UNBOXED <b>✳</b></span>
          </footer>
        </div>
      </section>
    </main>
  );
}
