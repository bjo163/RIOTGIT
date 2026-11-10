# R!OTGIT — Product Design System

## 1. Design thesis

**Extreme outside. Precise inside.**

R!OTGIT should feel like a punk print shop collided with a high-performance developer cockpit. The chrome can be loud; the code, data, and actions must remain calm and legible.

## 2. Visual modes

### Riot Dark (default)
- Coal canvas `#111111`.
- Asphalt elevated surfaces `#202020`.
- Concrete primary text `#F1F0E9`.
- Riot Lime `#D7FF38` for active brand signals.
- Hazard Orange `#FF5A1F` for selected actions and urgent highlights.

### Riot Light
- Concrete canvas `#F1F0E9`.
- White/paper surfaces with visible borders.
- Ink `#171717` primary text.
- Steel metadata.
- Lime and orange retained as accents, not large body surfaces.

Both modes must be first-class. Theme choice must not change semantic meaning.

## 3. Typography

Use a three-role system:
- **Display:** heavy condensed sans, uppercase, editorial. Preferred installed choices: Impact / system condensed stack, with robust fallbacks.
- **Interface:** neutral sans (system UI / Inter when available) for navigation, forms, prose.
- **Code:** monospace stack for source, hashes, commands, refs, and metadata.

Rules:
- Display type is for short labels and hero statements only.
- Body text must remain comfortable at 14–16 px minimum in dense surfaces.
- Hashes and paths may wrap or horizontally scroll; never truncate without a reveal path.
- Avoid all-caps paragraphs.
- Numerals in metrics should use tabular figures.

## 4. Shape language

- Hard corners and compact radii for panels, tags, buttons, and cards.
- Thick strokes, cut corners, offset rules, and occasional sticker-like callouts.
- No soft SaaS bubble-card overload.
- Use one dominant visual gesture per screen; avoid layering multiple grunge effects over every component.
- Shadows are structural, not decorative.

## 5. Layout

### Application shell
- Left rail / collapsible sidebar for primary navigation.
- Top utility bar for global search, command palette, create action, notifications, and account.
- Main workspace with strong page title, contextual metadata, and clear action cluster.
- Optional right inspector for repository details, reviewers, checks, or event history.
- Mobile uses a compact drawer and sticky page actions.

### Repository shell
- Repository identity, visibility, owner, default branch, and clone action are always easy to locate.
- Tabs: Code, Issues, Pull requests, Actions, Releases, Settings (only show enabled capabilities).
- Branch and path navigation remain distinct.
- Source view prioritizes readable line lengths, syntax colors, copy controls, and raw-file access.

### Dense engineering views
- Diff, logs, commits, and tables favor alignment and scanability.
- Keep status and author metadata stable.
- No animated glitch effects inside code or diffs.
- Provide keyboard navigation and accessible labels for icon-only actions.

## 6. Component rules

| Component | Design rule |
| --- | --- |
| Button | Squared, high-contrast, clear verb; destructive actions never use brand-lime styling |
| Input | Strong focus ring, persistent label, visible error text |
| Tabs | Underline or hard-edge active marker; keyboard accessible |
| Badge | Small semantic marker; never use as decoration for every item |
| Card/panel | Use only for meaningful grouping; strong border, restrained elevation |
| Modal | Confirm irreversible operations with explicit target and consequence |
| Table | Dense but readable; sortable headers expose state to assistive tech |
| Diff | Separate additions/deletions by semantics and text labels, not color alone |
| Code block | Monospace, line numbers optional, copy/raw actions available |
| Toast | Short, actionable, non-blocking; errors include recovery guidance |
| Empty state | One clear explanation + one primary next action |
| Command palette | Searchable, keyboard-first, permission-aware |

## 7. Interaction and motion

- Keyboard focus is always visible.
- Every action has loading, success, error, and disabled states where relevant.
- Optimistic updates must have rollback/reconciliation behavior.
- Never imply success before the server confirms it.
- Motion defaults to 120–180 ms for micro-interactions.
- Respect `prefers-reduced-motion`; disable glitch and parallax effects when requested.
- No flashing, strobing, or forced auto-playing sound.

## 8. Accessibility

Target WCAG 2.2 AA for all interactive product surfaces:
- text contrast at least 4.5:1 for normal text and 3:1 for large text;
- controls and focus indicators have sufficient contrast;
- keyboard-only completion for every workflow;
- visible labels and useful accessible names;
- semantic headings, landmarks, tables, and status announcements;
- errors identify the field and explain the fix;
- color is never the only carrier of state.

Brand colors may need darkened/lightened functional variants to pass contrast; do not use the raw palette blindly.

## 9. Responsive behavior

- Desktop: full rail, workspace, optional inspector.
- Tablet: compact rail and one-column workspace.
- Mobile: drawer, stacked metadata, horizontal repository tabs with overflow affordance, fixed primary action only when useful.
- Avoid tiny code text; provide horizontal scroll for code rather than crushing it.

## 10. Content rules

- Button labels start with verbs: Create, Push, Merge, Rebase, Archive, Restore.
- Use plain technical language in permissions and destructive actions.
- Show full repository path and target branch in risky operations.
- Distinguish "mirror", "replica", "federated node", and "P2P peer" precisely.
- Label experimental capabilities as Experimental.

## 11. Screen acceptance checklist

- [ ] Dark and light modes are complete.
- [ ] Display/Interface/Code type roles are consistent.
- [ ] Navigation hierarchy is predictable.
- [ ] Code and diffs remain highly readable.
- [ ] Keyboard and reduced-motion behavior works.
- [ ] Contrast and semantic state are tested.
- [ ] Mobile layouts remain usable.
- [ ] Grunge effects are restrained and never obstruct interaction.
