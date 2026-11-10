const repositories = [
  { name: "rift-engine", path: "riot-labs/rift-engine", language: "Rust", updated: "12 min ago", branch: "main", status: "clean" },
  { name: "noise-lab", path: "riot-labs/noise-lab", language: "TypeScript", updated: "48 min ago", branch: "main", status: "review" },
  { name: "ghost-protocol", path: "riot-labs/ghost-protocol", language: "Rust", updated: "2 hours ago", branch: "dev", status: "clean" },
];

const events = [
  { kind: "PUSH", title: "Pushed 4 commits to main", repo: "rift-engine", time: "12m" },
  { kind: "REVIEW", title: "Review requested", repo: "noise-lab", time: "48m" },
  { kind: "TAG", title: "Created release candidate", repo: "ghost-protocol", time: "2h" },
];

export default function Home() {
  return (
    <main className="shell">
      <aside className="rail">
        <a className="brand" href="/" aria-label="R!OTGIT home">
          <span className="brand-mark">R!</span>
          <span className="brand-name">OTGIT<span className="brand-dot">.</span></span>
        </a>
        <div className="rail-caption">THE NETWORK</div>
        <nav className="nav" aria-label="Main navigation">
          <a className="nav-item active" href="#overview"><span>▦</span> Overview</a>
          <a className="nav-item" href="#repositories"><span>⌘</span> Repositories</a>
          <a className="nav-item" href="#activity"><span>↯</span> Activity</a>
          <a className="nav-item" href="#network"><span>◉</span> Network</a>
        </nav>
        <div className="rail-caption rail-bottom-caption">WORKSPACE</div>
        <div className="workspace-chip"><span className="workspace-avatar">B</span><span><b>BUILDER HQ</b><small>Personal space</small></span><span className="chevron">⌄</span></div>
        <div className="rail-footer"><span className="live-dot" /> LOCAL NODE <span className="mono">01</span></div>
      </aside>

      <section className="main-area" id="overview">
        <header className="topbar">
          <div className="breadcrumb"><span>WORKSPACE</span><b>/</b><strong>OVERVIEW</strong></div>
          <div className="top-actions">
            <button className="search-button" type="button"><span>⌕</span> Search anything <kbd>⌘ K</kbd></button>
            <button className="icon-button" type="button" aria-label="Notifications">♧</button>
            <div className="user-avatar" aria-label="Account">B</div>
          </div>
        </header>

        <div className="content">
          <section className="hero">
            <div className="hero-grain" />
            <div className="hero-copy">
              <div className="eyebrow"><span className="live-dot" /> DESIGN PREVIEW / SAMPLE DATA</div>
              <h1>BREAK THE<br /><em>RULES.</em></h1>
              <p>Own your code. Build your network.<br />No permission required.</p>
              <div className="hero-actions">
                <a href="#repositories" className="button button-lime">EXPLORE REPOS <span>↗</span></a>
                <a href="#network" className="text-link">NODE STATUS <span>→</span></a>
              </div>
            </div>
            <div className="hero-stamp" aria-hidden="true">
              <div className="stamp-ring">SELF HOSTED · LOCAL FIRST · OPEN NETWORK ·</div>
              <div className="stamp-core">R!<small>OWN IT</small></div>
            </div>
            <div className="hero-index">ISSUE 001 <span>—</span> CODE WITHOUT PERMISSION</div>
          </section>

          <div className="section-heading">
            <div><div className="eyebrow muted">CONTROL ROOM / 01</div><h2>YOUR TERRITORY<span>_</span></h2></div>
            <a className="text-link" href="#repositories">ALL REPOSITORIES ↗</a>
          </div>

          <section className="stat-grid" aria-label="Workspace summary">
            <article className="stat-card"><span>REPOSITORIES</span><strong>03</strong><small>UNDER YOUR CONTROL</small></article>
            <article className="stat-card"><span>OPEN REVIEWS</span><strong>01</strong><small>NEEDS YOUR EYES</small></article>
            <article className="stat-card"><span>LOCAL NODE</span><strong className="stat-online">ONLINE<span className="live-dot" /></strong><small>127.0.0.1 : 8787</small></article>
            <article className="stat-card stat-card-accent"><span>NETWORK MODE</span><strong>LOCAL</strong><small>P2P ADAPTER DISABLED</small></article>
          </section>

          <div className="work-grid">
            <section id="repositories" className="panel repo-panel">
              <div className="panel-heading"><div><span className="eyebrow muted">SOURCE / 02</span><h3>RECENT REPOS</h3></div><button type="button" className="mini-action">＋ NEW REPO</button></div>
              <div className="repo-list">
                {repositories.map((repo) => (
                  <article className="repo-row" key={repo.name}>
                    <div className="repo-icon">⑂</div>
                    <div className="repo-info"><strong>{repo.name}</strong><span>{repo.path}</span></div>
                    <div className="repo-language"><i />{repo.language}</div>
                    <div className="repo-branch">{repo.branch}</div>
                    <div className={`repo-state ${repo.status}`}>{repo.status === "review" ? "REVIEW" : "CLEAN"}</div>
                    <div className="repo-updated">{repo.updated}</div>
                    <button type="button" className="row-more" aria-label={`More actions for ${repo.name}`}>···</button>
                  </article>
                ))}
              </div>
            </section>

            <section id="activity" className="panel activity-panel">
              <div className="panel-heading"><div><span className="eyebrow muted">SIGNAL / 03</span><h3>LIVE FEED</h3></div><span className="feed-live"><i /> LIVE</span></div>
              <div className="event-list">
                {events.map((event) => (
                  <article className="event-row" key={event.title}>
                    <span className="event-mark">{event.kind === "PUSH" ? "↗" : event.kind === "REVIEW" ? "◎" : "✳"}</span>
                    <div><b>{event.title}</b><span>{event.repo}</span></div>
                    <time>{event.time}</time>
                  </article>
                ))}
              </div>
              <a href="#activity" className="panel-link">OPEN ACTIVITY LOG ↗</a>
            </section>
          </div>

          <section id="network" className="network-strip">
            <div className="network-symbol">⌁</div>
            <div className="network-copy"><span className="eyebrow">NETWORK STATUS / EXPERIMENTAL LATER</span><strong>ONE NODE. FULL CONTROL.</strong><small>Mirrors, federation, and peer sync will arrive as optional adapters.</small></div>
            <span className="network-state"><i /> SINGLE NODE</span>
          </section>

          <footer className="footer"><span>R!OTGIT © 2026</span><span>THE UNDERGROUND CODE NETWORK</span><span>BUILT FOR THE UNBOXED <b>✳</b></span></footer>
        </div>
      </section>
    </main>
  );
}
