"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { type FormEvent, useCallback, useEffect, useState } from "react";

type BranchRule = {
  id: string;
  branch_name: string;
  require_pull_request: boolean;
  prevent_force_push: boolean;
  prevent_deletion: boolean;
  created_at: string;
  updated_at: string;
};

export default function BranchProtectionPage() {
  const params = useParams<{ owner: string; name: string }>();
  const owner = params.owner;
  const name = params.name;

  const [rules, setRules] = useState<BranchRule[]>([]);
  const [branchName, setBranchName] = useState("main");
  const [requirePullRequest, setRequirePullRequest] = useState(true);
  const [preventForcePush, setPreventForcePush] = useState(true);
  const [preventDeletion, setPreventDeletion] = useState(true);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");

  const loadRules = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const query = new URLSearchParams({ owner, name, resource: "branch-protection" });
      const response = await fetch("/api/repository-metadata?" + query.toString(), { cache: "no-store" });
      const payload = (await response.json().catch(() => ({}))) as BranchRule[] | { error?: string };
      if (!response.ok || !Array.isArray(payload)) {
        setError(!Array.isArray(payload) ? (payload.error ?? "Branch rules could not be loaded.") : "Branch rules could not be loaded.");
        return;
      }
      setRules(payload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name]);

  useEffect(() => {
    const timer = window.setTimeout(() => void loadRules(), 0);
    return () => window.clearTimeout(timer);
  }, [loadRules]);

  async function saveRule(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setSaving(true);
    setError("");
    setNotice("");
    try {
      const response = await fetch("/api/repository-metadata", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          resource: "branch-protection",
          owner,
          name,
          branch_name: branchName,
          require_pull_request: requirePullRequest,
          prevent_force_push: preventForcePush,
          prevent_deletion: preventDeletion,
        }),
      });
      const payload = (await response.json().catch(() => ({}))) as BranchRule & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Branch rule could not be saved.");
        return;
      }
      setRules((current) => [...current.filter((rule) => rule.branch_name !== payload.branch_name), payload]
        .sort((a, b) => a.branch_name.localeCompare(b.branch_name)));
      setNotice("BRANCH RULE SAVED. SMART HTTP ENFORCEMENT IS ACTIVE.");
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setSaving(false);
    }
  }

  async function removeRule(rule: BranchRule) {
    setError("");
    setNotice("");
    const query = new URLSearchParams({
      owner,
      name,
      resource: "branch-protection",
      branch_name: rule.branch_name,
    });
    try {
      const response = await fetch("/api/repository-metadata?" + query.toString(), { method: "DELETE" });
      if (!response.ok) {
        const payload = (await response.json().catch(() => ({}))) as { error?: string };
        setError(payload.error ?? "Branch rule could not be removed.");
        return;
      }
      setRules((current) => current.filter((item) => item.id !== rule.id));
      setNotice("BRANCH RULE REMOVED.");
    } catch {
      setError("Could not reach the R!OTGIT API.");
    }
  }

  function editRule(rule: BranchRule) {
    setBranchName(rule.branch_name);
    setRequirePullRequest(rule.require_pull_request);
    setPreventForcePush(rule.prevent_force_push);
    setPreventDeletion(rule.prevent_deletion);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  return (
    <main className="branch-settings-page">
      <header className="branch-settings-topbar">
        <Link href={`/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`} className="brand-small">
          <span>R!</span>OTGIT<span className="brand-small-dot">.</span>
        </Link>
        <div className="breadcrumb"><span>REPOSITORY</span><b>/</b><strong>BRANCH PROTECTION</strong></div>
        <Link href={`/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(name)}`} className="text-link">← BACK TO REPOSITORY</Link>
      </header>

      <div className="branch-settings-content">
        <div className="eyebrow muted">REPOSITORY CONTROL / SECURITY POLICY</div>
        <h1>PROTECT YOUR<br /><em>MAIN LINE.</em></h1>
        <p className="branch-settings-intro">
          Guard important refs at the Git receive-pack boundary. Rules are enforced by R!OTGIT Smart HTTP,
          not just by the browser.
        </p>

        {error ? <p className="login-error" role="alert">{error}</p> : null}
        {notice ? <p className="branch-settings-notice" role="status">{notice}</p> : null}

        <section className="branch-rule-builder">
          <div className="panel-heading">
            <div><span className="eyebrow muted">NEW OR UPDATE / RULE 01</span><h2>LOCK A BRANCH</h2></div>
            <span className="branch-builder-mark">R!</span>
          </div>
          <form onSubmit={saveRule}>
            <label className="branch-field-label" htmlFor="protected-branch">BRANCH NAME</label>
            <input
              id="protected-branch"
              className="branch-name-input"
              value={branchName}
              onChange={(event) => setBranchName(event.target.value)}
              maxLength={200}
              required
              placeholder="main"
            />
            <div className="branch-protection-options">
              <label className="branch-protection-option">
                <input type="checkbox" checked={requirePullRequest} onChange={(event) => setRequirePullRequest(event.target.checked)} />
                <span><strong>REQUIRE PULL REQUEST</strong><small>Block direct pushes. Changes must arrive through a reviewed merge workflow.</small></span>
              </label>
              <label className="branch-protection-option">
                <input type="checkbox" checked={preventForcePush} onChange={(event) => setPreventForcePush(event.target.checked)} />
                <span><strong>PREVENT FORCE PUSH</strong><small>Only allow fast-forward ref updates on this branch.</small></span>
              </label>
              <label className="branch-protection-option">
                <input type="checkbox" checked={preventDeletion} onChange={(event) => setPreventDeletion(event.target.checked)} />
                <span><strong>PREVENT DELETION</strong><small>Reject attempts to delete the protected branch ref.</small></span>
              </label>
            </div>
            <div className="branch-rule-actions">
              <button className="button button-lime" type="submit" disabled={saving || !branchName.trim() || (!requirePullRequest && !preventForcePush && !preventDeletion)}>
                {saving ? "SAVING..." : "SAVE BRANCH RULE"} <span>↗</span>
              </button>
              <span className="branch-rule-hint">ADMIN OR REPOSITORY ADMIN REQUIRED</span>
            </div>
          </form>
        </section>

        <section className="branch-rules-list">
          <div className="panel-heading">
            <div><span className="eyebrow muted">ACTIVE POLICIES / 02</span><h2>LOCKED REFS</h2></div>
            <button type="button" className="text-link" onClick={() => void loadRules()}>↻ REFRESH</button>
          </div>
          {loading ? (
            <div className="repo-browser-empty">READING BRANCH POLICIES...</div>
          ) : rules.length ? (
            <div className="branch-rule-list">
              {rules.map((rule) => (
                <article className="branch-rule-row" key={rule.id}>
                  <div className="branch-rule-ref"><span>⑂</span><strong>{rule.branch_name}</strong></div>
                  <div className="branch-rule-flags">
                    {rule.require_pull_request ? <span>PR REQUIRED</span> : null}
                    {rule.prevent_force_push ? <span>NO FORCE PUSH</span> : null}
                    {rule.prevent_deletion ? <span>NO DELETE</span> : null}
                  </div>
                  <div className="branch-rule-row-actions">
                    <button type="button" className="text-link" onClick={() => editRule(rule)}>EDIT</button>
                    <button type="button" className="text-link danger-link" onClick={() => void removeRule(rule)}>REMOVE</button>
                  </div>
                </article>
              ))}
            </div>
          ) : (
            <div className="repo-browser-empty">NO BRANCH RULES YET. MAIN IS UNPROTECTED.</div>
          )}
        </section>

        <aside className="branch-protection-warning">
          <strong>TRANSPORT COVERAGE / READ BEFORE ENABLING</strong>
          <p>
            Rules are enforced for authenticated Git Smart HTTP pushes. SSH transport is not implemented yet.
            A rule requiring a pull request blocks direct pushes, but the R!OTGIT pull-request merge engine is not available yet.
            Keep a recovery/admin path outside protected refs until that workflow ships.
          </p>
        </aside>
      </div>
    </main>
  );
}
