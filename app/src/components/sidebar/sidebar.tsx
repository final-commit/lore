'use client'

import { useState, useEffect, useCallback } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import {
  Search, ChevronRight, ChevronDown, FileText, Folder, FolderOpen,
  Star, Clock, BookOpen, Settings, Plus, Pin, LogOut, Layers,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { api } from '@/lib/api'
import type { TreeEntry, Collection, StarredDoc, ViewRecord } from '@/lib/api'
import { useAuth } from '@/contexts/auth-context'
import { NewDocDialog } from '../new-doc-dialog'
import { ThemeToggle } from '../theme-toggle'
import { NotificationBell } from '../notifications/notification-bell'

// ── Tree helpers ──────────────────────────────────────────────────────────────

interface TreeNode {
  name: string
  path: string
  is_dir: boolean
  children?: TreeNode[]
}

function buildTree(entries: TreeEntry[]): TreeNode[] {
  const root: TreeNode[] = []
  const dirs = new Map<string, TreeNode>()

  const sorted = [...entries].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1
    return a.path.localeCompare(b.path)
  })

  for (const entry of sorted) {
    const parts = entry.path.split('/')
    if (parts.length === 1) {
      const node: TreeNode = { name: entry.name, path: entry.path, is_dir: entry.is_dir, children: entry.is_dir ? [] : undefined }
      root.push(node)
      if (entry.is_dir) dirs.set(entry.path, node)
    } else {
      const parentPath = parts.slice(0, -1).join('/')
      const parent = dirs.get(parentPath)
      if (parent) {
        const node: TreeNode = { name: entry.name, path: entry.path, is_dir: entry.is_dir, children: entry.is_dir ? [] : undefined }
        parent.children = parent.children ?? []
        parent.children.push(node)
        if (entry.is_dir) dirs.set(entry.path, node)
      }
    }
  }
  return root
}

function titleFromPath(path: string): string {
  const name = path.split('/').pop() ?? path
  return name.replace(/\.md$/, '').replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}

// ── Section toggle ────────────────────────────────────────────────────────────

function SectionHeader({
  label, collapsed, onToggle, action,
}: {
  label: string
  collapsed: boolean
  onToggle: () => void
  action?: React.ReactNode
}) {
  return (
    <div className="flex items-center justify-between px-2 py-1">
      <button
        onClick={onToggle}
        className="flex items-center gap-1 text-[10px] font-semibold uppercase tracking-wider text-zinc-500 hover:text-zinc-300 transition-colors"
      >
        {collapsed ? <ChevronRight className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
        {label}
      </button>
      {action}
    </div>
  )
}

// ── Doc tree (recursive) ──────────────────────────────────────────────────────

function FlatDocTree({ nodes, level = 0, onAddToCollection }: {
  nodes: TreeNode[]
  level?: number
  onAddToCollection?: (path: string) => void
}) {
  return (
    <ul className={cn('space-y-0.5', level > 0 && 'ml-3 border-l border-zinc-800 pl-2')}>
      {nodes.map((node) => (
        <FlatDocTreeItem key={node.path} node={node} level={level} onAddToCollection={onAddToCollection} />
      ))}
    </ul>
  )
}

function FlatDocTreeItem({ node, level, onAddToCollection }: {
  node: TreeNode
  level: number
  onAddToCollection?: (path: string) => void
}) {
  const pathname = usePathname()
  const [expanded, setExpanded] = useState(level === 0)
  const [hovered, setHovered] = useState(false)
  const title = titleFromPath(node.name)
  const href = `/docs/${node.path.replace(/\.md$/, '')}`
  const isActive = pathname === href

  if (node.is_dir) {
    return (
      <li>
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex w-full items-center gap-1.5 rounded px-2 py-1 text-sm text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200 transition-colors"
        >
          <ChevronRight className={cn('h-3 w-3 shrink-0 transition-transform', expanded && 'rotate-90')} />
          {expanded ? <FolderOpen className="h-4 w-4 shrink-0" /> : <Folder className="h-4 w-4 shrink-0" />}
          <span className="truncate">{title}</span>
        </button>
        {expanded && node.children && <FlatDocTree nodes={node.children} level={level + 1} onAddToCollection={onAddToCollection} />}
      </li>
    )
  }

  return (
    <li
      className="group relative"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <Link
        href={href}
        className={cn(
          'flex items-center gap-1.5 rounded px-2 py-1 text-sm transition-colors',
          isActive
            ? 'bg-zinc-800 font-medium text-zinc-100'
            : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200',
        )}
      >
        <FileText className="h-4 w-4 shrink-0 text-zinc-500" />
        <span className="flex-1 truncate">{title}</span>
      </Link>
      {hovered && onAddToCollection && (
        <button
          onClick={() => onAddToCollection(node.path)}
          title="Add to collection"
          className="absolute right-1 top-1/2 -translate-y-1/2 rounded p-0.5 text-zinc-600 hover:bg-zinc-700 hover:text-zinc-300"
        >
          <Layers className="h-3 w-3" />
        </button>
      )}
    </li>
  )
}

// ── User avatar ───────────────────────────────────────────────────────────────

function UserAvatar({ name, role }: { name: string; role: string }) {
  const initials = name.split(' ').map((w) => w[0]).join('').slice(0, 2).toUpperCase()
  const roleColor = role === 'admin' ? 'bg-amber-600' : role === 'editor' ? 'bg-blue-600' : 'bg-zinc-600'
  return (
    <div className="flex items-center gap-2.5">
      <div className={cn('flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[11px] font-semibold text-white', roleColor)}>
        {initials}
      </div>
      <div className="min-w-0">
        <p className="truncate text-sm font-medium text-zinc-200">{name}</p>
        <span className={cn(
          'rounded px-1 py-0.5 text-[9px] font-semibold uppercase tracking-wide',
          role === 'admin' ? 'bg-amber-900/50 text-amber-400' :
          role === 'editor' ? 'bg-blue-900/50 text-blue-400' :
          'bg-zinc-800 text-zinc-400',
        )}>
          {role}
        </span>
      </div>
    </div>
  )
}

// ── Sidebar ───────────────────────────────────────────────────────────────────

interface SidebarProps {
  tree?: TreeEntry[]
  className?: string
}

export function Sidebar({ tree: externalTree, className }: SidebarProps) {
  const { user, logout } = useAuth()
  const router = useRouter()
  const pathname = usePathname()

  const [treeEntries, setTreeEntries] = useState<TreeEntry[]>(externalTree ?? [])
  const [collections, setCollections] = useState<Collection[]>([])
  const [starred, setStarred] = useState<StarredDoc[]>([])
  const [recent, setRecent] = useState<ViewRecord[]>([])

  const [collectionsOpen, setCollectionsOpen] = useState(true)
  const [starredOpen, setStarredOpen] = useState(true)
  const [recentOpen, setRecentOpen] = useState(false)
  const [docsOpen, setDocsOpen] = useState(true)

  const [showSearch, setShowSearch] = useState(false)
  const [newCollectionOpen, setNewCollectionOpen] = useState(false)

  const treeNodes = buildTree(treeEntries)

  const load = useCallback(async () => {
    try {
      const [entries, cols, stars, views] = await Promise.all([
        api.docs.tree(),
        api.collections.list().catch(() => [] as Collection[]),
        api.stars.list().catch(() => [] as StarredDoc[]),
        api.views.recent().catch(() => [] as ViewRecord[]),
      ])
      setTreeEntries(entries)
      setCollections(cols)
      setStarred(stars)
      setRecent(views.slice(0, 5))
    } catch { /* ignore */ }
  }, [])

  useEffect(() => { load() }, [load])

  // Open search via ⌘K (emit event that SearchDialog listens to)
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault()
        setShowSearch(true)
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [])

  const handleAddToCollection = (path: string) => {
    // Future: show dialog to pick collection
    console.log('Add to collection:', path)
  }

  return (
    <aside className={cn(
      'flex h-full w-64 flex-col border-r border-zinc-800 bg-zinc-950',
      className,
    )}>
      {/* Header: Logo + notifications + settings */}
      <div className="flex items-center gap-2 border-b border-zinc-800 px-3 py-2.5">
        <Link href="/docs" className="flex items-center gap-1.5 flex-1 min-w-0">
          <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded bg-blue-600">
            <BookOpen className="h-3.5 w-3.5 text-white" />
          </div>
          <span className="text-sm font-semibold text-zinc-100">Lore</span>
        </Link>
        <NotificationBell />
        <Link
          href="/settings/profile"
          className="rounded p-1 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
          title="Settings"
        >
          <Settings className="h-4 w-4" />
        </Link>
      </div>

      {/* User section */}
      {user && (
        <div className="border-b border-zinc-800 px-3 py-2.5">
          <UserAvatar name={user.name} role={user.role} />
        </div>
      )}

      {/* Search */}
      <div className="px-3 py-2">
        <button
          onClick={() => {
            const event = new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true })
            document.dispatchEvent(event)
          }}
          className="flex w-full items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-500 transition-colors hover:border-zinc-700 hover:text-zinc-400"
        >
          <Search className="h-3.5 w-3.5 shrink-0" />
          <span className="flex-1 text-left">Search...</span>
          <kbd className="rounded border border-zinc-700 bg-zinc-800 px-1 py-0.5 text-[10px] text-zinc-500">⌘K</kbd>
        </button>
      </div>

      {/* Scrollable nav */}
      <nav className="flex-1 overflow-y-auto px-2 py-1 space-y-1">

        {/* Collections */}
        <div>
          <SectionHeader
            label="Collections"
            collapsed={!collectionsOpen}
            onToggle={() => setCollectionsOpen(!collectionsOpen)}
            action={
              <button
                onClick={() => setNewCollectionOpen(true)}
                className="rounded p-0.5 text-zinc-600 hover:text-zinc-400 hover:bg-zinc-800 transition-colors"
                title="New collection"
              >
                <Plus className="h-3.5 w-3.5" />
              </button>
            }
          />
          {collectionsOpen && (
            <ul className="mt-0.5 space-y-0.5">
              {collections.length === 0 ? (
                <li className="px-2 py-1 text-xs text-zinc-600">No collections yet</li>
              ) : (
                collections.map((col) => (
                  <li key={col.id}>
                    <Link
                      href={`/collections/${col.id}`}
                      className={cn(
                        'flex items-center gap-2 rounded px-2 py-1.5 text-sm transition-colors',
                        pathname.startsWith(`/collections/${col.id}`)
                          ? 'bg-zinc-800 text-zinc-100'
                          : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200',
                      )}
                    >
                      <span className="text-base leading-none">{col.emoji ?? '📁'}</span>
                      <span className="flex-1 truncate">{col.name}</span>
                      <span className="text-[10px] text-zinc-600">{col.doc_count}</span>
                    </Link>
                  </li>
                ))
              )}
            </ul>
          )}
        </div>

        {/* Starred */}
        <div>
          <SectionHeader
            label="Starred"
            collapsed={!starredOpen}
            onToggle={() => setStarredOpen(!starredOpen)}
          />
          {starredOpen && (
            <ul className="mt-0.5 space-y-0.5">
              {starred.length === 0 ? (
                <li className="px-2 py-1 text-xs text-zinc-600">No starred docs</li>
              ) : (
                starred.map((doc) => (
                  <li key={doc.path}>
                    <Link
                      href={`/docs/${doc.path.replace(/\.md$/, '')}`}
                      className={cn(
                        'flex items-center gap-1.5 rounded px-2 py-1 text-sm transition-colors',
                        pathname === `/docs/${doc.path.replace(/\.md$/, '')}`
                          ? 'bg-zinc-800 text-zinc-100'
                          : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200',
                      )}
                    >
                      <Star className="h-3.5 w-3.5 shrink-0 fill-amber-400 text-amber-400" />
                      <span className="truncate">{doc.title || titleFromPath(doc.path)}</span>
                    </Link>
                  </li>
                ))
              )}
            </ul>
          )}
        </div>

        {/* Recent */}
        <div>
          <SectionHeader
            label="Recent"
            collapsed={!recentOpen}
            onToggle={() => setRecentOpen(!recentOpen)}
          />
          {recentOpen && (
            <ul className="mt-0.5 space-y-0.5">
              {recent.length === 0 ? (
                <li className="px-2 py-1 text-xs text-zinc-600">No recent docs</li>
              ) : (
                recent.map((view) => (
                  <li key={view.path}>
                    <Link
                      href={`/docs/${view.path.replace(/\.md$/, '')}`}
                      className={cn(
                        'flex items-center gap-1.5 rounded px-2 py-1 text-sm transition-colors',
                        pathname === `/docs/${view.path.replace(/\.md$/, '')}`
                          ? 'bg-zinc-800 text-zinc-100'
                          : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200',
                      )}
                    >
                      <Clock className="h-3.5 w-3.5 shrink-0 text-zinc-600" />
                      <span className="truncate">{view.title || titleFromPath(view.path)}</span>
                    </Link>
                  </li>
                ))
              )}
            </ul>
          )}
        </div>

        {/* All docs */}
        <div>
          <SectionHeader
            label="Documents"
            collapsed={!docsOpen}
            onToggle={() => setDocsOpen(!docsOpen)}
            action={<NewDocDialog />}
          />
          {docsOpen && (
            <div className="mt-0.5">
              <FlatDocTree nodes={treeNodes} onAddToCollection={handleAddToCollection} />
            </div>
          )}
        </div>
      </nav>

      {/* Footer */}
      <div className="flex items-center gap-2 border-t border-zinc-800 px-3 py-2">
        <ThemeToggle />
        <Link
          href="/trash"
          className="rounded p-1 text-zinc-600 hover:text-zinc-400 hover:bg-zinc-800 transition-colors"
          title="Trash"
        >
          <span className="text-xs">🗑</span>
        </Link>
        <div className="flex-1" />
        {user && (
          <button
            onClick={logout}
            className="flex items-center gap-1 rounded px-2 py-1 text-xs text-zinc-600 hover:text-zinc-400 hover:bg-zinc-800 transition-colors"
            title="Sign out"
          >
            <LogOut className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      {/* New collection dialog */}
      {newCollectionOpen && (
        <NewCollectionDialog
          onClose={() => setNewCollectionOpen(false)}
          onCreate={(col) => {
            setCollections((prev) => [...prev, col])
            setNewCollectionOpen(false)
            router.push(`/collections/${col.id}`)
          }}
        />
      )}
    </aside>
  )
}

// ── New collection dialog ─────────────────────────────────────────────────────

const EMOJIS = ['📁', '📚', '🗂️', '📋', '🔖', '💡', '🛠️', '🔬', '🎯', '📊', '🌐', '🚀']
const COLORS = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ef4444', '#06b6d4', '#f97316', '#84cc16']

function NewCollectionDialog({ onClose, onCreate }: {
  onClose: () => void
  onCreate: (col: Collection) => void
}) {
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [emoji, setEmoji] = useState('📁')
  const [color, setColor] = useState(COLORS[0])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleCreate = async () => {
    if (!name.trim()) return
    setLoading(true)
    setError(null)
    try {
      const col = await api.collections.create({ name, description: description || undefined, emoji, color })
      onCreate(col)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to create collection')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl">
        <h2 className="mb-4 text-sm font-semibold text-zinc-200">New Collection</h2>

        <div className="space-y-4">
          {/* Emoji picker */}
          <div>
            <label className="mb-1.5 block text-xs text-zinc-500">Icon</label>
            <div className="flex flex-wrap gap-1">
              {EMOJIS.map((e) => (
                <button
                  key={e}
                  onClick={() => setEmoji(e)}
                  className={cn(
                    'h-8 w-8 rounded text-lg transition-colors',
                    emoji === e ? 'bg-zinc-700 ring-1 ring-blue-500' : 'hover:bg-zinc-800',
                  )}
                >
                  {e}
                </button>
              ))}
            </div>
          </div>

          {/* Color picker */}
          <div>
            <label className="mb-1.5 block text-xs text-zinc-500">Color</label>
            <div className="flex gap-1.5">
              {COLORS.map((c) => (
                <button
                  key={c}
                  onClick={() => setColor(c)}
                  style={{ backgroundColor: c }}
                  className={cn(
                    'h-6 w-6 rounded-full transition-transform',
                    color === c ? 'scale-125 ring-2 ring-white ring-offset-1 ring-offset-zinc-900' : 'hover:scale-110',
                  )}
                />
              ))}
            </div>
          </div>

          <div>
            <label className="mb-1.5 block text-xs text-zinc-500">Name</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Engineering Docs"
              autoFocus
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
              onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            />
          </div>

          <div>
            <label className="mb-1.5 block text-xs text-zinc-500">Description (optional)</label>
            <input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Technical documentation for the team"
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
            />
          </div>

          {error && <p className="text-xs text-red-400">{error}</p>}

          <div className="flex justify-end gap-2 pt-1">
            <button onClick={onClose} className="px-3 py-1.5 text-xs text-zinc-500 hover:text-zinc-300">Cancel</button>
            <button
              onClick={handleCreate}
              disabled={!name.trim() || loading}
              className={cn(
                'rounded-lg px-4 py-1.5 text-xs font-medium transition-colors',
                name.trim() && !loading
                  ? 'bg-blue-600 text-white hover:bg-blue-500'
                  : 'bg-zinc-700 text-zinc-500 cursor-not-allowed',
              )}
            >
              {loading ? 'Creating...' : 'Create'}
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
