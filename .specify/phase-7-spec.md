# Phase 7: Projects — Spec

## Goal
Replace Linear. Close the loop on the full team OS.

## Tasks

### 7.1 — Issue Tracker
- `projects` table: id, workspace_id, name, settings
- `issues` table: id, project_id, title, body (rich text), status, priority, assignee_id, created_by (user|agent), milestone_id, labels[], created_at, updated_at
- `milestones` table: id, project_id, title, due_date, description
- `issue_labels` table: id, project_id, name, color

### 7.2 — REST API for Projects
- Full CRUD for projects, issues, milestones, labels
- `GET /api/projects/:id/issues?status=&assignee=&label=&milestone=`
- `POST /api/projects/:id/issues` — create issue
- `PUT /api/issues/:id` — update
- `DELETE /api/issues/:id`
- `POST /api/issues/:id/comments` — comment on issue
- `GET /api/issues/:id/activity` — activity log

### 7.3 — Kanban Board UI
- `/projects/:id/board` — kanban view
- Columns: backlog, in-progress, in-review, done (configurable)
- Drag-and-drop cards (react-beautiful-dnd or dnd-kit)
- Swimlanes by assignee or milestone
- WIP limits (optional, configurable)
- Issue detail panel (side panel, not full page navigation)

### 7.4 — List + Filter View
- `/projects/:id/issues` — table/list view
- Filter by: status, assignee, label, milestone, created_by
- Sort by: created, updated, priority, due date
- Bulk actions: assign, label, change status, delete

### 7.5 — GitHub Issue Sync (Bidirectional)
- Connect GitHub repo to Lore project
- On Lore issue create → create GitHub issue via API
- On GitHub issue event (webhook) → sync to Lore issue
- Map: status open/closed, assignee (by email match), labels, milestones
- Conflict resolution: last-write-wins with audit trail

### 7.6 — Agent Integration for Issues
- Agents can create, update, close issues (CreateIssues capability)
- Issues can trigger agents: `@agent-name` in issue body → agent triggered
- Agent run results can auto-create issues (failing test → bug issue)
- `triage-agent` built-in: on issue create → reads body → sets labels + priority + assignee suggestion

### 7.7 — Unified Search
- Issues indexed in Tantivy alongside docs + messages + code
- `GET /api/search?q=&types=docs,messages,code,issues`

### 7.8 — Tests
- Issue CRUD, status transitions
- Kanban drag-and-drop state changes (API level)
- GitHub sync: mock webhook → verify issue synced
- Agent creates issue → verify in DB
- All existing tests pass

## Acceptance Criteria
- [ ] Issue tracker: create, assign, label, milestone, close
- [ ] Kanban board with drag-and-drop
- [ ] GitHub bidirectional sync
- [ ] Agent can create/update issues
- [ ] triage-agent built-in works
- [ ] Unified search includes issues
- [ ] All tests pass
- [ ] git commit: "feat: Phase 7 — projects (issues, kanban, milestones, GitHub sync)"
