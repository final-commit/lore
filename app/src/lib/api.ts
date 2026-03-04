/**
 * API client for the Lore Rust backend.
 * Uses REST endpoints with Bearer token auth.
 */

const API_BASE = typeof window !== 'undefined' ? '' : (process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:3000')

function getAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' }
  if (typeof window !== 'undefined') {
    const token = localStorage.getItem('forge_token')
    if (token) headers['Authorization'] = `Bearer ${token}`
  }
  return headers
}

async function get<T>(path: string, params?: Record<string, string>): Promise<T> {
  const url = new URL(path, API_BASE || 'http://localhost:3000')
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      if (v != null) url.searchParams.set(k, v)
    })
  }
  const res = await fetch(url.toString(), { headers: getAuthHeaders() })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `API error: ${res.status}`)
  }
  return res.json()
}

async function post<T>(path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${API_BASE || 'http://localhost:3000'}${path}`, {
    method: 'POST',
    headers: getAuthHeaders(),
    body: body ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    throw new Error(data.error || `API error: ${res.status}`)
  }
  return res.json()
}

async function put<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${API_BASE || 'http://localhost:3000'}${path}`, {
    method: 'PUT',
    headers: getAuthHeaders(),
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    throw new Error(data.error || `API error: ${res.status}`)
  }
  return res.json()
}

async function del(path: string): Promise<void> {
  const res = await fetch(`${API_BASE || 'http://localhost:3000'}${path}`, {
    method: 'DELETE',
    headers: getAuthHeaders(),
  })
  if (!res.ok) {
    const data = await res.json().catch(() => ({}))
    throw new Error(data.error || `API error: ${res.status}`)
  }
}

async function download(path: string, params?: Record<string, string>): Promise<Blob> {
  const url = new URL(path, API_BASE || 'http://localhost:3000')
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      if (v != null) url.searchParams.set(k, v)
    })
  }
  const res = await fetch(url.toString(), { headers: getAuthHeaders() })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `API error: ${res.status}`)
  }
  return res.blob()
}

// ── Types (matching Rust backend responses) ──────────────────────────────────

export interface DocResponse {
  path: string
  content: string
  sha: string
  commit_sha: string
}

export interface TreeEntry {
  path: string
  name: string
  is_dir: boolean
  sha: string
}

export interface CommitInfo {
  sha: string
  message: string
  author: string
  author_email: string
  timestamp: number
}

export interface Comment {
  id: string
  doc_path: string
  parent_id: string | null
  author_id: string
  author_name?: string
  body: string
  anchor_text: string | null
  anchor_start: number | null
  anchor_end: number | null
  resolved_at: string | null
  resolved_by: string | null
  is_agent: boolean
  agent_name?: string
  reactions?: Record<string, number>
  created_at: string
  updated_at: string
}

export interface SearchResult {
  path: string
  title: string
  snippet: string
  score: number
}

export interface AuthResponse {
  access_token: string
  refresh_token: string
  user: UserInfo
}

export interface UserInfo {
  id: string
  email: string
  name: string
  role: string
  created_at: string
}

export interface SyncStatus {
  last_pull_at: string | null
  last_push_at: string | null
  last_pull_commit: string | null
  last_push_commit: string | null
  remote_url: string | null
  branch: string
}

// ── New types ─────────────────────────────────────────────────────────────────

export interface Collection {
  id: string
  name: string
  description: string | null
  emoji: string | null
  color: string | null
  owner_id: string
  member_count: number
  doc_count: number
  created_at: string
  updated_at: string
}

export interface CollectionDoc {
  path: string
  title: string
  order: number
  added_at: string
}

export interface StarredDoc {
  path: string
  title: string
  starred_at: string
}

export interface DocStatus {
  path: string
  published: boolean
  published_at: string | null
  published_by: string | null
  is_draft: boolean
  is_pinned: boolean
}

export interface Notification {
  id: string
  type: string
  title: string
  body: string
  doc_path: string | null
  read_at: string | null
  created_at: string
  actor_name?: string
}

export interface ShareSettings {
  enabled: boolean
  url_id: string | null
  include_children: boolean
  created_at: string | null
}

export interface SharedDoc {
  path: string
  title: string
  content: string
  include_children: boolean
  children?: SharedDoc[]
}

export interface Preferences {
  theme: 'dark' | 'light' | 'system'
  language: string
  notifications_email: boolean
  notifications_in_app: boolean
  editor_font_size: number
  editor_spell_check: boolean
}

export interface TeamSettings {
  name: string
  allow_signup: boolean
  default_role: string
  require_email_verification: boolean
}

export interface FullUserInfo {
  id: string
  email: string
  name: string
  role: string
  created_at: string
  last_active?: string
}

export interface Group {
  id: string
  name: string
  description: string | null
  member_count: number
  created_at: string
}

export interface GroupMember {
  user_id: string
  name: string
  email: string
  role: string
  added_at: string
}

export interface ApiToken {
  id: string
  name: string
  token?: string // only on creation
  prefix: string
  created_at: string
  last_used?: string | null
  expires_at?: string | null
}

export interface Shortcut {
  action: string
  description: string
  keys: string[]
  category: string
}

export interface ViewRecord {
  path: string
  title: string
  viewed_at: string
}

export interface TrashedDoc {
  path: string
  title: string
  trashed_at: string
  trashed_by: string
}

export interface ArchivedDoc {
  path: string
  title: string
  archived_at: string
  archived_by: string
}

export interface DiffResult {
  old_content: string
  new_content: string
  hunks: DiffHunk[]
}

export interface DiffHunk {
  old_start: number
  old_lines: number
  new_start: number
  new_lines: number
  lines: DiffLine[]
}

export interface DiffLine {
  kind: 'context' | 'added' | 'removed'
  content: string
}

export interface OAuthProvider {
  provider: string
  enabled: boolean
  configured: boolean
  client_id?: string
}

export interface UnfurlResult {
  url: string
  title?: string
  description?: string
  image?: string
  favicon?: string
  embed_html?: string
  embed_width?: number
  embed_height?: number
  site_name?: string
}

export interface CustomEmoji {
  id: string
  name: string
  shortcode: string
  url: string
  created_by: string
  created_at: string
}

export interface AiSuggestion {
  original: string
  suggestion: string
  explanation?: string
}

export interface AiAnswer {
  answer: string
  sources?: string[]
}

export interface AiSummary {
  summary: string
}

export interface AiGenerated {
  content: string
}

export interface ExportJob {
  id: string
  type: string
  status: 'pending' | 'running' | 'done' | 'failed'
  collection_id?: string
  created_at: string
  completed_at?: string
  error?: string
  file_size?: number
}

// ── API ──────────────────────────────────────────────────────────────────────

export const api = {
  auth: {
    register: (email: string, name: string, password: string) =>
      post<AuthResponse>('/api/auth/register', { email, name, password }),
    login: (email: string, password: string) =>
      post<AuthResponse>('/api/auth/login', { email, password }),
    refresh: (refresh_token: string) =>
      post<AuthResponse>('/api/auth/refresh', { refresh_token }),
    me: () => get<UserInfo>('/api/auth/me'),
    providers: () => get<OAuthProvider[]>('/api/auth/providers'),
    configureProvider: (provider: string, data: { client_id: string; client_secret: string; enabled: boolean }) =>
      put<OAuthProvider>(`/api/auth/providers/${provider}`, data),
  },
  docs: {
    tree: () => get<TreeEntry[]>('/api/tree'),
    get: (path: string) => get<DocResponse>(`/api/docs/${encodeURIComponent(path)}`),
    create: (path: string, content: string, message?: string) =>
      post<{ path: string; commit_sha: string }>('/api/docs', { path, content, message }),
    update: (path: string, content: string, message?: string) =>
      put<{ commit_sha: string }>(`/api/docs/${encodeURIComponent(path)}`, { content, message }),
    delete: (path: string) => del(`/api/docs/${encodeURIComponent(path)}`),
    history: (path: string) => get<CommitInfo[]>(`/api/docs/${encodeURIComponent(path)}/history`),
    diff: (path: string, fromSha: string, toSha: string) =>
      get<DiffResult>(`/api/docs/${encodeURIComponent(path)}/diff`, { from: fromSha, to: toSha }),
    status: (path: string) => get<DocStatus>(`/api/docs/${encodeURIComponent(path)}/status`),
    publish: (path: string) => post<DocStatus>(`/api/doc-publish/${encodeURIComponent(path)}`),
    unpublish: (path: string) => post<DocStatus>(`/api/doc-unpublish/${encodeURIComponent(path)}`),
    trash: (path: string) => post<void>(`/api/docs/${encodeURIComponent(path)}/trash`),
    restore: (path: string) => post<void>(`/api/docs/${encodeURIComponent(path)}/restore`),
    permanentDelete: (path: string) => del(`/api/docs/${encodeURIComponent(path)}/permanent`),
    archive: (path: string) => post<void>(`/api/docs/${encodeURIComponent(path)}/archive`),
    unarchive: (path: string) => post<void>(`/api/docs/${encodeURIComponent(path)}/unarchive`),
    listTrash: () => get<TrashedDoc[]>('/api/docs/trash'),
    listArchive: () => get<ArchivedDoc[]>('/api/docs/archive'),
    exportDoc: (path: string, format: 'markdown' | 'html') =>
      download('/api/export/doc', { path, format }),
  },
  comments: {
    list: (doc_path: string) => get<Comment[]>('/api/comments', { doc_path }),
    create: (doc_path: string, body: string, opts?: { parent_id?: string; anchor_text?: string; anchor_start?: number; anchor_end?: number }) =>
      post<Comment>('/api/comments', { doc_path, body, ...opts }),
    update: (id: string, body: string) => put<Comment>(`/api/comments/${id}`, { body }),
    delete: (id: string) => del(`/api/comments/${id}`),
    resolve: (id: string) => post<Comment>(`/api/comments/${id}/resolve`),
    unresolve: (id: string) => post<Comment>(`/api/comments/${id}/unresolve`),
    react: (id: string, emoji: string) => post<Comment>(`/api/comments/${id}/react`, { emoji }),
  },
  search: {
    query: (q: string, limit?: number) =>
      get<SearchResult[]>('/api/search', { q, ...(limit ? { limit: String(limit) } : {}) }),
  },
  sync: {
    status: () => get<SyncStatus>('/api/sync/status'),
    pull: () => post<{ success: boolean; message: string }>('/api/sync/pull'),
    push: () => post<{ success: boolean; message: string }>('/api/sync/push'),
  },
  collections: {
    list: () => get<Collection[]>('/api/collections'),
    get: (id: string) => get<Collection>(`/api/collections/${id}`),
    create: (data: { name: string; description?: string; emoji?: string; color?: string }) =>
      post<Collection>('/api/collections', data),
    update: (id: string, data: Partial<{ name: string; description: string; emoji: string; color: string }>) =>
      put<Collection>(`/api/collections/${id}`, data),
    delete: (id: string) => del(`/api/collections/${id}`),
    getDocs: (id: string) => get<CollectionDoc[]>(`/api/collections/${id}/docs`),
    addDoc: (id: string, path: string) => post<void>(`/api/collections/${id}/docs`, { path }),
    removeDoc: (id: string, path: string) => del(`/api/collections/${id}/docs/${encodeURIComponent(path)}`),
    reorderDoc: (id: string, path: string, order: number) =>
      put<void>(`/api/collections/${id}/docs/${encodeURIComponent(path)}/order`, { order }),
  },
  stars: {
    list: () => get<StarredDoc[]>('/api/stars'),
    toggle: (path: string) => post<{ starred: boolean }>('/api/stars', { path }),
    check: (path: string) => get<{ starred: boolean }>('/api/stars/check', { path }),
  },
  notifications: {
    list: () => get<Notification[]>('/api/notifications'),
    markRead: (id: string) => post<void>(`/api/notifications/${id}/read`),
    markAllRead: () => post<void>('/api/notifications/read-all'),
    unreadCount: () => get<{ count: number }>('/api/notifications/unread-count'),
  },
  share: {
    get: (path: string) => get<ShareSettings>('/api/share', { path }),
    create: (path: string, include_children: boolean) =>
      post<ShareSettings>('/api/share', { path, include_children }),
    update: (path: string, data: Partial<{ enabled: boolean; include_children: boolean }>) =>
      put<ShareSettings>('/api/share', { path, ...data }),
    getPublic: (url_id: string) => get<SharedDoc>(`/api/shared/${url_id}`),
  },
  preferences: {
    get: () => get<Preferences>('/api/preferences'),
    update: (prefs: Partial<Preferences>) => put<Preferences>('/api/preferences', prefs),
  },
  settings: {
    get: () => get<TeamSettings>('/api/settings'),
    update: (data: Partial<TeamSettings>) => put<TeamSettings>('/api/settings', data),
  },
  users: {
    list: () => get<FullUserInfo[]>('/api/users'),
    search: (q: string) => get<FullUserInfo[]>('/api/users/search', { q }),
    updateRole: (id: string, role: string) => put<FullUserInfo>(`/api/users/${id}/role`, { role }),
    invite: (email: string, role: string) => post<void>('/api/users/invite', { email, role }),
  },
  groups: {
    list: () => get<Group[]>('/api/groups'),
    get: (id: string) => get<Group>(`/api/groups/${id}`),
    create: (data: { name: string; description?: string }) => post<Group>('/api/groups', data),
    update: (id: string, data: Partial<{ name: string; description: string }>) =>
      put<Group>(`/api/groups/${id}`, data),
    delete: (id: string) => del(`/api/groups/${id}`),
    getMembers: (id: string) => get<GroupMember[]>(`/api/groups/${id}/members`),
    addMember: (id: string, user_id: string) => post<void>(`/api/groups/${id}/members`, { user_id }),
    removeMember: (id: string, user_id: string) => del(`/api/groups/${id}/members/${user_id}`),
  },
  tokens: {
    list: () => get<ApiToken[]>('/api/api-tokens'),
    create: (name: string, expires_at?: string) =>
      post<ApiToken>('/api/api-tokens', { name, expires_at }),
    delete: (id: string) => del(`/api/api-tokens/${id}`),
  },
  shortcuts: {
    list: () => get<Shortcut[]>('/api/shortcuts'),
  },
  views: {
    recent: () => get<ViewRecord[]>('/api/views/recent'),
    record: (path: string) => post<void>('/api/views/recent', { path }),
  },
  unfurl: {
    url: (url: string) => get<UnfurlResult>('/api/unfurl', { url }),
  },
  emojis: {
    list: () => get<CustomEmoji[]>('/api/emojis'),
    create: (data: { name: string; shortcode: string; url: string }) =>
      post<CustomEmoji>('/api/emojis', data),
    delete: (id: string) => del(`/api/emojis/${id}`),
  },
  ai: {
    suggest: (doc_path: string, content: string) =>
      post<AiSuggestion[]>('/api/ai/suggest', { doc_path, content }),
    answer: (doc_path: string, question: string) =>
      post<AiAnswer>('/api/ai/answer', { doc_path, question }),
    summarize: (content: string) =>
      post<AiSummary>('/api/ai/summarize', { content }),
    generate: (outline: string) =>
      post<AiGenerated>('/api/ai/generate', { outline }),
  },
  exportJobs: {
    create: (type: string, collection_id?: string) =>
      post<ExportJob>('/api/export-jobs', { type, collection_id }),
    get: (id: string) => get<ExportJob>(`/api/export-jobs/${id}`),
    download: (id: string) => download(`/api/export-jobs/${id}/download`),
  },
  health: () => get<{ status: string; db: string; git: { status: string; head: string | null }; version: string }>('/health'),
}

// ── Auth helpers ─────────────────────────────────────────────────────────────

export function setTokens(access: string, refresh: string) {
  if (typeof window !== 'undefined') {
    localStorage.setItem('forge_token', access)
    localStorage.setItem('forge_refresh_token', refresh)
  }
}

export function clearTokens() {
  if (typeof window !== 'undefined') {
    localStorage.removeItem('forge_token')
    localStorage.removeItem('forge_refresh_token')
  }
}

export function getAccessToken(): string | null {
  if (typeof window !== 'undefined') {
    return localStorage.getItem('forge_token')
  }
  return null
}

export function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}
