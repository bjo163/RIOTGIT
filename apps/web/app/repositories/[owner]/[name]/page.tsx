"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { type FormEvent, useCallback, useEffect, useState } from "react";

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

type DiffResult = {
  base: string;
  head: string;
  diff: string;
};

type PullRequestEntry = {
  id: string;
  number: number;
  title: string;
  body: string | null;
  state: "open" | "closed" | "merging" | "merged";
  head_ref: string;
  base_ref: string;
  head_sha: string;
  base_sha: string;
  author_id: string;
  merged_by: string | null;
  merge_commit: string | null;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  merged_at: string | null;
};

type PullRequestReviewEntry = {
  id: string;
  reviewer_id: string;
  state: "comment" | "approved" | "changes_requested";
  body: string | null;
  submitted_at: string;
};

type IssueEntry = {
  id: string;
  number: number;
  title: string;
  body: string | null;
  state: "open" | "closed";
  author_id: string;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  milestone_id: string | null;
};

type LabelEntry = {
  id: string;
  name: string;
  color: string;
  description: string | null;
};

type MilestoneEntry = {
  id: string;
  number: number;
  title: string;
  description: string | null;
  state: string;
  due_on: string | null;
};

type IssueCommentEntry = {
  id: string;
  issue_number: number;
  author_id: string;
  body: string;
  created_at: string;
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
  const [pullRequests, setPullRequests] = useState<PullRequestEntry[]>([]);
  const [selectedPullRequest, setSelectedPullRequest] = useState<PullRequestEntry | null>(null);
  const [pullRequestReviews, setPullRequestReviews] = useState<PullRequestReviewEntry[]>([]);
  const [pullRequestTitle, setPullRequestTitle] = useState("");
  const [pullRequestBody, setPullRequestBody] = useState("");
  const [pullRequestHead, setPullRequestHead] = useState("");
  const [pullRequestBase, setPullRequestBase] = useState("main");
  const [reviewState, setReviewState] = useState<"approved" | "changes_requested" | "comment">("approved");
  const [reviewBody, setReviewBody] = useState("");
  const [pullRequestBusy, setPullRequestBusy] = useState(false);
  const [issues, setIssues] = useState<IssueEntry[]>([]);
  const [labels, setLabels] = useState<LabelEntry[]>([]);
  const [milestones, setMilestones] = useState<MilestoneEntry[]>([]);
  const [issueLabels, setIssueLabels] = useState<LabelEntry[]>([]);
  const [selectedLabelId, setSelectedLabelId] = useState("");
  const [newLabelName, setNewLabelName] = useState("");
  const [newLabelColor, setNewLabelColor] = useState("D7FF38");
  const [newMilestoneTitle, setNewMilestoneTitle] = useState("");
  const [newMilestoneDue, setNewMilestoneDue] = useState("");
  const [activeIssueNumber, setActiveIssueNumber] = useState<number | null>(null);
  const [issueComments, setIssueComments] = useState<IssueCommentEntry[]>([]);
  const [commentDraft, setCommentDraft] = useState("");
  const [commentsLoading, setCommentsLoading] = useState(false);
  const [issueTitle, setIssueTitle] = useState("");
  const [issueBody, setIssueBody] = useState("");
  const [diffBase, setDiffBase] = useState("main");
  const [diffHead, setDiffHead] = useState("HEAD");
  const [diffResult, setDiffResult] = useState<DiffResult | null>(null);
  const [view, setView] = useState<"code" | "history" | "issues" | "diff" | "pulls">("code");
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

  const loadDiff = useCallback(async () => {
    setView("diff");
    setLoading(true);
    setError("");
    try {
      const query = new URLSearchParams({
        resource: "diff",
        owner,
        name,
        base: diffBase,
        head: diffHead,
      });
      const response = await fetch("/api/repository-data?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as DiffResult & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Could not load repository diff.");
        setDiffResult(null);
        return;
      }
      setDiffResult(payload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name, diffBase, diffHead]);

  const loadPullRequests = useCallback(async () => {
    setView("pulls");
    setLoading(true);
    setError("");
    setSelectedPullRequest(null);
    setPullRequestReviews([]);
    try {
      const query = new URLSearchParams({ owner, name });
      const response = await fetch("/api/repository-pulls?" + query.toString(), { cache: "no-store" });
      const payload = (await response.json().catch(() => ({}))) as PullRequestEntry[] | { error?: string };
      if (!response.ok || !Array.isArray(payload)) {
        setError(!Array.isArray(payload) ? (payload.error ?? "Pull requests could not be loaded.") : "Pull requests could not be loaded.");
        return;
      }
      setPullRequests(payload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name]);

  async function openPullRequest(number: number) {
    setPullRequestBusy(true);
    setError("");
    try {
      const baseQuery = new URLSearchParams({ owner, name, number: String(number), resource: "detail" });
      const reviewsQuery = new URLSearchParams({ owner, name, number: String(number), resource: "reviews" });
      const [pullResponse, reviewsResponse] = await Promise.all([
        fetch("/api/repository-pulls?" + baseQuery.toString(), { cache: "no-store" }),
        fetch("/api/repository-pulls?" + reviewsQuery.toString(), { cache: "no-store" }),
      ]);
      const pullPayload = (await pullResponse.json().catch(() => ({}))) as PullRequestEntry & { error?: string };
      const reviewsPayload = (await reviewsResponse.json().catch(() => [])) as PullRequestReviewEntry[] | { error?: string };
      if (!pullResponse.ok) {
        setError(pullPayload.error ?? "Pull request could not be loaded.");
        return;
      }
      setSelectedPullRequest(pullPayload);
      if (reviewsResponse.ok && Array.isArray(reviewsPayload)) setPullRequestReviews(reviewsPayload);
      else setPullRequestReviews([]);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setPullRequestBusy(false);
    }
  }

  async function submitPullRequest(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setPullRequestBusy(true);
    try {
      const response = await fetch("/api/repository-pulls", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          resource: "create",
          owner,
          name,
          title: pullRequestTitle,
          body: pullRequestBody,
          head_ref: pullRequestHead,
          base_ref: pullRequestBase,
        }),
      });
      const payload = (await response.json().catch(() => ({}))) as PullRequestEntry & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Pull request could not be opened.");
        return;
      }
      setPullRequestTitle("");
      setPullRequestBody("");
      setSelectedPullRequest(payload);
      await loadPullRequests();
      setSelectedPullRequest(payload);
      setView("pulls");
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setPullRequestBusy(false);
    }
  }

  async function submitPullRequestReview(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedPullRequest) return;
    setError("");
    setPullRequestBusy(true);
    try {
      const response = await fetch("/api/repository-pulls", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          resource: "review",
          owner,
          name,
          number: selectedPullRequest.number,
          state: reviewState,
          body: reviewBody,
        }),
      });
      const payload = (await response.json().catch(() => ({}))) as PullRequestReviewEntry & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Review could not be submitted.");
        return;
      }
      setReviewBody("");
      await openPullRequest(selectedPullRequest.number);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setPullRequestBusy(false);
    }
  }

  async function mergePullRequest() {
    if (!selectedPullRequest) return;
    setError("");
    setPullRequestBusy(true);
    try {
      const response = await fetch("/api/repository-pulls", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ resource: "merge", owner, name, number: selectedPullRequest.number }),
      });
      const payload = (await response.json().catch(() => ({}))) as PullRequestEntry & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Pull request could not be merged.");
        return;
      }
      setSelectedPullRequest(payload);
      setPullRequests((current) => current.map((item) => item.number === payload.number ? payload : item));
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setPullRequestBusy(false);
    }
  }

  async function changePullRequestState() {
    if (!selectedPullRequest) return;
    const nextState = selectedPullRequest.state === "open" ? "closed" : "open";
    if (!window.confirm(`${nextState === "closed" ? "Close" : "Reopen"} pull request #${selectedPullRequest.number}?`)) return;
    setError("");
    setPullRequestBusy(true);
    try {
      const response = await fetch("/api/repository-pulls", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ owner, name, number: selectedPullRequest.number, state: nextState }),
      });
      const payload = (await response.json().catch(() => ({}))) as PullRequestEntry & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Pull request state could not be updated.");
        return;
      }
      setSelectedPullRequest(payload);
      setPullRequests((current) => current.map((item) => item.number === payload.number ? payload : item));
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setPullRequestBusy(false);
    }
  }

  async function viewPullRequestDiff(pull: PullRequestEntry) {
    setError("");
    setLoading(true);
    setView("diff");
    setDiffBase(pull.base_ref);
    setDiffHead(pull.head_ref);
    try {
      const query = new URLSearchParams({
        resource: "diff",
        owner,
        name,
        base: pull.base_ref,
        head: pull.head_ref,
      });
      const response = await fetch("/api/repository-data?" + query.toString(), { cache: "no-store" });
      const payload = (await response.json().catch(() => ({}))) as DiffResult & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Pull request diff could not be loaded.");
        setDiffResult(null);
        return;
      }
      setDiffResult(payload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }

  const loadIssues = useCallback(async () => {
    setView("issues");
    setLoading(true);
    setError("");
    try {
      const query = new URLSearchParams({ owner, name });
      const response = await fetch("/api/repository-issues?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as IssueEntry[] | { error?: string };
      if (!response.ok || !Array.isArray(payload)) {
        setError(!Array.isArray(payload) ? (payload.error ?? "Could not load issues.") : "Could not load issues.");
        return;
      }
      setIssues(payload);
      const metadataBase = "/api/repository-metadata?owner=" + encodeURIComponent(owner) + "&name=" + encodeURIComponent(name);
      const [labelsResponse, milestonesResponse] = await Promise.all([
        fetch(metadataBase + "&resource=labels", { cache: "no-store" }),
        fetch(metadataBase + "&resource=milestones", { cache: "no-store" }),
      ]);
      const labelsPayload = (await labelsResponse.json().catch(() => [])) as LabelEntry[] | { error?: string };
      const milestonesPayload = (await milestonesResponse.json().catch(() => [])) as MilestoneEntry[] | { error?: string };
      if (labelsResponse.ok && Array.isArray(labelsPayload)) setLabels(labelsPayload);
      if (milestonesResponse.ok && Array.isArray(milestonesPayload)) setMilestones(milestonesPayload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setLoading(false);
    }
  }, [owner, name]);

  async function openIssueComments(issueNumber: number) {
    if (activeIssueNumber === issueNumber) {
      setActiveIssueNumber(null);
      setIssueComments([]);
      setCommentDraft("");
      return;
    }
    setActiveIssueNumber(issueNumber);
    setIssueComments([]);
    setCommentDraft("");
    setCommentsLoading(true);
    const query = new URLSearchParams({ owner, name, number: String(issueNumber) });
    try {
      const response = await fetch("/api/repository-issues?" + query.toString(), {
        cache: "no-store",
      });
      const payload = (await response.json().catch(() => ({}))) as IssueCommentEntry[] | { error?: string };
      if (!response.ok || !Array.isArray(payload)) {
        setError(!Array.isArray(payload) ? (payload.error ?? "Could not load issue comments.") : "Could not load issue comments.");
        return;
      }
      setIssueComments(payload);
      const labelQuery = new URLSearchParams({ owner, name, number: String(issueNumber), resource: "issue-labels" });
      const labelResponse = await fetch("/api/repository-metadata?" + labelQuery.toString(), { cache: "no-store" });
      const labelPayload = (await labelResponse.json().catch(() => [])) as LabelEntry[] | { error?: string };
      if (labelResponse.ok && Array.isArray(labelPayload)) setIssueLabels(labelPayload);
    } catch {
      setError("Could not reach the R!OTGIT API.");
    } finally {
      setCommentsLoading(false);
    }
  }

  async function submitIssueComment(event: FormEvent<HTMLFormElement>, issueNumber: number) {
    event.preventDefault();
    if (!commentDraft.trim()) return;
    setError("");
    try {
      const response = await fetch("/api/repository-issues", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ owner, name, number: issueNumber, body: commentDraft }),
      });
      const payload = (await response.json().catch(() => ({}))) as IssueCommentEntry & { error?: string };
      if (!response.ok) {
        setError(payload.error ?? "Comment could not be posted.");
        return;
      }
      setCommentDraft("");
      const query = new URLSearchParams({ owner, name, number: String(issueNumber) });
      const commentsResponse = await fetch("/api/repository-issues?" + query.toString(), { cache: "no-store" });
      const commentsPayload = (await commentsResponse.json().catch(() => ({}))) as IssueCommentEntry[] | { error?: string };
      if (commentsResponse.ok && Array.isArray(commentsPayload)) {
        setIssueComments(commentsPayload);
        setActiveIssueNumber(issueNumber);
      } else {
        setError(!Array.isArray(commentsPayload) ? (commentsPayload.error ?? "Comment was posted but comments could not be refreshed.") : "Comment was posted but comments could not be refreshed.");
      }
    } catch {
      setError("Could not reach the R!OTGIT API.");
    }
  }

  async function createRepositoryLabel(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    const response = await fetch("/api/repository-metadata", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        resource: "labels",
        owner,
        name,
        label_name: newLabelName,
        color: newLabelColor,
      }),
    });
    const payload = (await response.json().catch(() => ({}))) as LabelEntry & { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Label could not be created.");
      return;
    }
    setLabels((current) => [...current, payload].sort((a, b) => a.name.localeCompare(b.name)));
    setNewLabelName("");
  }

  async function createRepositoryMilestone(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    const response = await fetch("/api/repository-metadata", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        resource: "milestones",
        owner,
        name,
        title: newMilestoneTitle,
        due_on: newMilestoneDue || undefined,
      }),
    });
    const payload = (await response.json().catch(() => ({}))) as MilestoneEntry & { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Milestone could not be created.");
      return;
    }
    setMilestones((current) => [payload, ...current]);
    setNewMilestoneTitle("");
    setNewMilestoneDue("");
  }

  async function changeIssueMilestone(issue: IssueEntry, value: string) {
    const response = value
      ? await fetch("/api/repository-metadata", {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ resource: "issue-milestone", owner, name, number: issue.number, milestone_number: Number(value) }),
        })
      : await fetch("/api/repository-metadata?owner=" + encodeURIComponent(owner) + "&name=" + encodeURIComponent(name) + "&resource=issue-milestone&number=" + issue.number, { method: "DELETE" });
    const payload = (await response.json().catch(() => ({}))) as { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Milestone assignment failed.");
      return;
    }
    const milestone = milestones.find((item) => item.number === Number(value));
    setIssues((current) => current.map((item) => item.number === issue.number ? { ...item, milestone_id: milestone?.id ?? null } : item));
  }

  async function addIssueLabel(issueNumber: number) {
    if (!selectedLabelId) return;
    const response = await fetch("/api/repository-metadata", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ resource: "issue-label", owner, name, number: issueNumber, label_id: selectedLabelId }),
    });
    const payload = (await response.json().catch(() => ({}))) as { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Label could not be added.");
      return;
    }
    const query = new URLSearchParams({ owner, name, number: String(issueNumber), resource: "issue-labels" });
    const refreshed = await fetch("/api/repository-metadata?" + query.toString(), { cache: "no-store" });
    const refreshedPayload = (await refreshed.json().catch(() => [])) as LabelEntry[];
    if (refreshed.ok && Array.isArray(refreshedPayload)) setIssueLabels(refreshedPayload);
    setSelectedLabelId("");
  }

  async function removeIssueLabel(issueNumber: number, labelId: string) {
    const query = new URLSearchParams({ owner, name, number: String(issueNumber), resource: "issue-label", label_id: labelId });
    const response = await fetch("/api/repository-metadata?" + query.toString(), { method: "DELETE" });
    if (!response.ok) {
      const payload = (await response.json().catch(() => ({}))) as { error?: string };
      setError(payload.error ?? "Label could not be removed.");
      return;
    }
    setIssueLabels((current) => current.filter((label) => label.id !== labelId));
  }

  async function submitIssue(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    const response = await fetch("/api/repository-issues", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ owner, name, title: issueTitle, body: issueBody }),
    });
    const payload = (await response.json().catch(() => ({}))) as IssueEntry & { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Issue could not be created.");
      return;
    }
    setIssueTitle("");
    setIssueBody("");
    await loadIssues();
  }

  async function toggleIssue(issue: IssueEntry) {
    setError("");
    const response = await fetch("/api/repository-issues", {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        owner,
        name,
        number: issue.number,
        state: issue.state === "open" ? "closed" : "open",
      }),
    });
    const payload = (await response.json().catch(() => ({}))) as IssueEntry & { error?: string };
    if (!response.ok) {
      setError(payload.error ?? "Issue state could not be updated.");
      return;
    }
    setIssues((current) => current.map((item) => item.number === issue.number ? payload : item));
  }

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
          <Link href={`/repositories/${encodeURIComponent(owner)}/${encodeURIComponent(name)}/settings/branches`} className="text-link">BRANCH RULES ↗</Link>
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
        <button
          type="button"
          className={view === "issues" ? "repo-tab active" : "repo-tab"}
          onClick={() => void loadIssues()}
        >◎ ISSUES <small>{issues.filter((issue) => issue.state === "open").length}</small></button>
        <button
          type="button"
          className={view === "pulls" ? "repo-tab active" : "repo-tab"}
          onClick={() => void loadPullRequests()}
        >⇢ PULLS <small>{pullRequests.filter((pull) => pull.state === "open" || pull.state === "merging").length}</small></button>
        <button
          type="button"
          className={view === "diff" ? "repo-tab active" : "repo-tab"}
          onClick={() => void loadDiff()}
        >± DIFF</button>
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
      ) : view === "history" ? (
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
      ) : view === "pulls" ? (
        <section className="repo-browser-panel pull-requests-workspace">
          <form className="pr-create" onSubmit={submitPullRequest}>
            <div className="issue-create-heading">
              <span className="eyebrow muted">OPEN A REVIEW / NEW PULL REQUEST</span>
              <strong>PROPOSE A CHANGE.</strong>
            </div>
            <label htmlFor="pr-title">TITLE</label>
            <input
              id="pr-title"
              value={pullRequestTitle}
              onChange={(event) => setPullRequestTitle(event.target.value)}
              maxLength={200}
              required
              placeholder="What does this change do?"
            />
            <div className="pr-branch-fields">
              <label htmlFor="pr-head">FROM BRANCH
                <input id="pr-head" value={pullRequestHead} onChange={(event) => setPullRequestHead(event.target.value)} maxLength={200} required placeholder="feature/my-change" />
              </label>
              <span>→</span>
              <label htmlFor="pr-base">INTO BRANCH
                <input id="pr-base" value={pullRequestBase} onChange={(event) => setPullRequestBase(event.target.value)} maxLength={200} required />
              </label>
            </div>
            <label htmlFor="pr-body">DESCRIPTION</label>
            <textarea
              id="pr-body"
              value={pullRequestBody}
              onChange={(event) => setPullRequestBody(event.target.value)}
              maxLength={100000}
              rows={3}
              placeholder="Context, implementation notes, and verification..."
            />
            <button className="mini-action issue-submit" type="submit" disabled={pullRequestBusy || !pullRequestTitle.trim() || !pullRequestHead.trim() || !pullRequestBase.trim()}>
              ＋ OPEN PULL REQUEST
            </button>
          </form>

          <div className="pr-workspace-heading">
            <strong>REVIEW QUEUE</strong>
            <span>{pullRequests.filter((pull) => pull.state === "open").length} OPEN / {pullRequests.length} TOTAL</span>
            <button type="button" className="text-link" onClick={() => void loadPullRequests()}>↻ REFRESH</button>
          </div>

          {loading ? (
            <div className="repo-browser-empty">LOADING PULL REQUESTS...</div>
          ) : pullRequests.length ? (
            <div className="pr-layout">
              <div className="pr-list">
                {pullRequests.map((pull) => (
                  <button
                    type="button"
                    className={selectedPullRequest?.number === pull.number ? "pr-list-row selected" : "pr-list-row"}
                    key={pull.id}
                    onClick={() => void openPullRequest(pull.number)}
                  >
                    <span className={`pr-state ${pull.state}`}>{pull.state === "merged" ? "✓" : pull.state === "closed" ? "×" : pull.state === "merging" ? "↻" : "⇢"}</span>
                    <span className="pr-list-copy">
                      <strong>#{pull.number} {pull.title}</strong>
                      <small>{pull.head_ref} → {pull.base_ref}</small>
                    </span>
                    <span className="pr-list-state">{pull.state.toUpperCase()}</span>
                  </button>
                ))}
              </div>

              {selectedPullRequest ? (
                <article className="pr-detail">
                  {pullRequestBusy ? <div className="pr-busy-line">UPDATING REVIEW STATE...</div> : null}
                  <div className="pr-detail-heading">
                    <div>
                      <span className="eyebrow muted">PULL REQUEST / #{selectedPullRequest.number}</span>
                      <h3>{selectedPullRequest.title}</h3>
                      <small>{selectedPullRequest.state.toUpperCase()} · OPENED {new Date(selectedPullRequest.created_at).toLocaleDateString()} · BY {selectedPullRequest.author_id.slice(0, 8)}</small>
                    </div>
                    <span className={`pr-state large ${selectedPullRequest.state}`}>{selectedPullRequest.state.toUpperCase()}</span>
                  </div>
                  {selectedPullRequest.body ? <p className="pr-description">{selectedPullRequest.body}</p> : null}
                  <div className="pr-ref-grid">
                    <div><small>HEAD</small><strong>{selectedPullRequest.head_ref}</strong><code>{selectedPullRequest.head_sha.slice(0, 12)}</code></div>
                    <span>→</span>
                    <div><small>BASE</small><strong>{selectedPullRequest.base_ref}</strong><code>{selectedPullRequest.base_sha.slice(0, 12)}</code></div>
                  </div>
                  <div className="pr-detail-actions">
                    <button type="button" className="text-link" onClick={() => void viewPullRequestDiff(selectedPullRequest)}>VIEW DIFF ↗</button>
                    {selectedPullRequest.state === "open" ? (
                      <>
                        <button type="button" className="mini-action" disabled={pullRequestBusy} onClick={() => void mergePullRequest()}>MERGE PULL REQUEST</button>
                        <button type="button" className="text-link" disabled={pullRequestBusy} onClick={() => void changePullRequestState()}>CLOSE</button>
                      </>
                    ) : selectedPullRequest.state === "closed" ? (
                      <button type="button" className="text-link" disabled={pullRequestBusy} onClick={() => void changePullRequestState()}>REOPEN</button>
                    ) : null}
                  </div>
                  {selectedPullRequest.state === "merging" ? (
                    <p className="pr-merge-warning">MERGE RECOVERY IS PENDING. Refresh this pull request to reconcile the branch ref and metadata.</p>
                  ) : null}
                  {selectedPullRequest.state === "merged" && selectedPullRequest.merge_commit ? (
                    <p className="pr-merged-note">MERGED AT {selectedPullRequest.merge_commit.slice(0, 12)}</p>
                  ) : null}

                  <section className="pr-review-section">
                    <div className="pr-section-title"><strong>REVIEW HISTORY</strong><span>{pullRequestReviews.length} SUBMISSIONS</span></div>
                    {pullRequestReviews.length ? pullRequestReviews.map((review) => (
                      <article className="pr-review-row" key={review.id}>
                        <span className={`review-state ${review.state}`}>{review.state === "approved" ? "✓" : review.state === "changes_requested" ? "!" : "·"}</span>
                        <div><strong>{review.reviewer_id.slice(0, 8)} / {review.state.replace("_", " ").toUpperCase()}</strong><small>{new Date(review.submitted_at).toLocaleString()}</small>{review.body ? <p>{review.body}</p> : null}</div>
                      </article>
                    )) : <div className="repo-browser-empty compact">NO REVIEWS YET.</div>}
                  </section>

                  {selectedPullRequest.state === "open" ? (
                    <form className="pr-review-form" onSubmit={submitPullRequestReview}>
                      <label htmlFor="pr-review-state">SUBMIT REVIEW</label>
                      <select id="pr-review-state" value={reviewState} onChange={(event) => setReviewState(event.target.value as "approved" | "changes_requested" | "comment")}>
                        <option value="approved">APPROVE</option>
                        <option value="changes_requested">REQUEST CHANGES</option>
                        <option value="comment">COMMENT ONLY</option>
                      </select>
                      <textarea value={reviewBody} onChange={(event) => setReviewBody(event.target.value)} maxLength={20000} rows={3} placeholder="Review notes, risks, and suggestions..." />
                      <button className="mini-action" type="submit" disabled={pullRequestBusy}>SUBMIT REVIEW</button>
                    </form>
                  ) : null}
                </article>
              ) : (
                <div className="pr-detail-empty">SELECT A PULL REQUEST TO REVIEW THE CHANGESET.</div>
              )}
            </div>
          ) : (
            <div className="repo-browser-empty">NO PULL REQUESTS YET. OPEN A REVIEW TO START.</div>
          )}
        </section>
      ) : view === "issues" ? (
        <section className="repo-browser-panel issues-workspace">
          <form className="issue-create" onSubmit={submitIssue}>
            <div className="issue-create-heading">
              <span className="eyebrow muted">OPEN A SIGNAL / NEW ISSUE</span>
              <strong>WHAT NEEDS TO BE BUILT?</strong>
            </div>
            <label htmlFor="issue-title">TITLE</label>
            <input
              id="issue-title"
              value={issueTitle}
              onChange={(event) => setIssueTitle(event.target.value)}
              maxLength={200}
              required
              placeholder="Describe the problem in one clear line"
            />
            <label htmlFor="issue-body">DETAILS</label>
            <textarea
              id="issue-body"
              value={issueBody}
              onChange={(event) => setIssueBody(event.target.value)}
              maxLength={100000}
              rows={4}
              placeholder="Context, expected behavior, and acceptance criteria..."
            />
            <button className="mini-action issue-submit" type="submit" disabled={!issueTitle.trim()}>
              ＋ CREATE ISSUE
            </button>
          </form>
          <div className="issue-metadata-tools">
            <form className="metadata-create-form" onSubmit={createRepositoryLabel}>
              <strong className="eyebrow muted">DEFINE LABEL</strong>
              <input value={newLabelName} onChange={(event) => setNewLabelName(event.target.value)} maxLength={50} placeholder="label name" aria-label="New label name" required />
              <input value={newLabelColor} onChange={(event) => setNewLabelColor(event.target.value)} maxLength={7} placeholder="D7FF38" aria-label="Label color hex" />
              <button type="submit" className="mini-action" disabled={!newLabelName.trim()}>＋ LABEL</button>
            </form>
            <form className="metadata-create-form" onSubmit={createRepositoryMilestone}>
              <strong className="eyebrow muted">CREATE MILESTONE</strong>
              <input value={newMilestoneTitle} onChange={(event) => setNewMilestoneTitle(event.target.value)} maxLength={100} placeholder="milestone title" aria-label="New milestone title" required />
              <input type="date" value={newMilestoneDue} onChange={(event) => setNewMilestoneDue(event.target.value)} aria-label="Milestone due date" />
              <button type="submit" className="mini-action" disabled={!newMilestoneTitle.trim()}>＋ MILESTONE</button>
            </form>
          </div>
          <div className="issues-heading">
            <strong>ISSUE TRACKER</strong>
            <span>{issues.filter((issue) => issue.state === "open").length} OPEN / {issues.length} TOTAL</span>
            <button type="button" className="text-link" onClick={() => void loadIssues()}>↻ REFRESH</button>
          </div>
          {loading ? (
            <div className="repo-browser-empty">LOADING ISSUE TRACKER...</div>
          ) : issues.length ? (
            <div className="issue-list">
              {issues.map((issue) => (
                <article className="issue-row" key={issue.id}>
                  <span className={issue.state === "open" ? "issue-state open" : "issue-state closed"}>
                    {issue.state === "open" ? "◎" : "✓"}
                  </span>
                  <div className="issue-main">
                    <strong>#{issue.number} {issue.title}</strong>
                    <small>{issue.state.toUpperCase()} · OPENED {new Date(issue.created_at).toLocaleDateString()}</small>
                    {issue.body ? <p>{issue.body}</p> : null}
                  </div>
                  <div className="issue-main-meta">
                    <label>
                      MILESTONE
                      <select
                        value={milestones.find((item) => item.id === issue.milestone_id)?.number.toString() ?? ""}
                        onChange={(event) => void changeIssueMilestone(issue, event.target.value)}
                        aria-label={`Milestone for issue ${issue.number}`}
                      >
                        <option value="">NO MILESTONE</option>
                        {milestones.filter((item) => item.state === "open").map((milestone) => (
                          <option value={milestone.number} key={milestone.id}>{milestone.number}. {milestone.title}</option>
                        ))}
                      </select>
                    </label>
                  </div>
                  <div className="issue-actions">
                    <button type="button" className="text-link issue-toggle" onClick={() => void openIssueComments(issue.number)}>
                      {activeIssueNumber === issue.number ? "HIDE COMMENTS" : "COMMENTS"}
                    </button>
                    <button type="button" className="text-link issue-toggle" onClick={() => void toggleIssue(issue)}>
                      {issue.state === "open" ? "CLOSE" : "REOPEN"}
                    </button>
                  </div>
                  {activeIssueNumber === issue.number ? (
                    <div className="issue-comments">
                      <strong className="eyebrow muted">DISCUSSION / #{issue.number}</strong>
                      <div className="issue-label-editor">
                        <span className="eyebrow muted">LABELS</span>
                        <div className="issue-label-chips">
                          {issueLabels.map((label) => (
                            <button
                              type="button"
                              key={label.id}
                              className="issue-label-chip"
                              style={{ borderColor: "#" + label.color }}
                              onClick={() => void removeIssueLabel(issue.number, label.id)}
                              title="Remove label"
                            >{label.name} ×</button>
                          ))}
                        </div>
                        <div className="issue-label-add">
                          <select value={selectedLabelId} onChange={(event) => setSelectedLabelId(event.target.value)} aria-label="Choose label">
                            <option value="">CHOOSE LABEL</option>
                            {labels.filter((label) => !issueLabels.some((applied) => applied.id === label.id)).map((label) => (
                              <option value={label.id} key={label.id}>{label.name}</option>
                            ))}
                          </select>
                          <button type="button" className="text-link" disabled={!selectedLabelId} onClick={() => void addIssueLabel(issue.number)}>ADD</button>
                        </div>
                      </div>

                      {commentsLoading ? (
                        <span className="issue-comment-empty">LOADING COMMENTS...</span>
                      ) : issueComments.length ? (
                        issueComments.map((comment) => (
                          <article className="issue-comment" key={comment.id}>
                            <span className="comment-author">{comment.author_id.slice(0, 8)}</span>
                            <div><p>{comment.body}</p><small>{new Date(comment.created_at).toLocaleString()}</small></div>
                          </article>
                        ))
                      ) : (
                        <span className="issue-comment-empty">NO COMMENTS YET.</span>
                      )}
                      {issue.state === "open" ? (
                        <form className="issue-comment-form" onSubmit={(event) => void submitIssueComment(event, issue.number)}>
                          <textarea
                            aria-label="Write a comment"
                            rows={2}
                            value={commentDraft}
                            onChange={(event) => setCommentDraft(event.target.value)}
                            maxLength={10000}
                            placeholder="Add a useful comment..."
                          />
                          <button type="submit" className="mini-action" disabled={!commentDraft.trim()}>POST COMMENT</button>
                        </form>
                      ) : null}
                    </div>
                  ) : null}
                </article>
              ))}
            </div>
          ) : (
            <div className="repo-browser-empty">NO ISSUES YET. FIND THE FIRST THING TO FIX.</div>
          )}
        </section>
      ) : (
        <section className="repo-browser-panel diff-workspace">
          <div className="diff-controls">
            <label>
              BASE
              <input value={diffBase} onChange={(event) => setDiffBase(event.target.value)} maxLength={256} />
            </label>
            <span>→</span>
            <label>
              COMPARE
              <input value={diffHead} onChange={(event) => setDiffHead(event.target.value)} maxLength={256} />
            </label>
            <button type="button" className="mini-action" onClick={() => void loadDiff()}>RUN DIFF ↗</button>
          </div>
          {loading ? (
            <div className="repo-browser-empty">COMPARING GIT OBJECTS...</div>
          ) : diffResult?.diff ? (
            <pre className="diff-code"><code>{diffResult.diff}</code></pre>
          ) : (
            <div className="repo-browser-empty">NO DIFFERENCES FOUND — OR THE REFS ARE NOT AVAILABLE.</div>
          )}
        </section>
      )}
    </main>
  );
}
