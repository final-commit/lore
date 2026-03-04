'use client'

import { useMemo, useState, useEffect, useRef } from 'react'
import { Clock, GitCommit, Edit3, MessageSquare, History, Star, Share2, Globe, Circle, Download, ChevronDown, FileText, Archive } from 'lucide-react'
import { cn } from '@/lib/utils'
import { api, triggerDownload } from '@/lib/api'
import type { DocResponse, DocStatus } from '@/lib/api'
import { useAuth } from '@/contexts/auth-context'
import { ShareDialog } from './sharing/share-dialog'
import { useToast } from '@/contexts/toast-context'

interface DocViewerProps {
  doc: DocResponse & { title?: string; updatedAt?: string; lastCommit?: string }
  className?: string
  onEdit?: () => void
  onToggleComments?: () => void
  onToggleHistory?: () => void
}

export function DocViewer({ doc, className, onEdit, onToggleComments, onToggleHistory }: DocViewerProps) {
  const { user } = useAuth()
  const { toast } = useToast()
  const [starred, setStarred] = useState(false)
  const [starLoading, setStarLoading] = useState(false)
  const [status, setStatus] = useState<DocStatus | null>(null)
  const [publishLoading, setPublishLoading] = useState(false)
  const [shareOpen, setShareOpen] = useState(false)
  const [exportOpen, setExportOpen] = useState(false)
  const [exporting, setExporting] = useState(false)
  const exportRef = useRef<HTMLDivElement>(null)

  const isEditorOrAdmin = user && (user.role === 'editor' || user.role === 'admin')

  // Close export dropdown on outside click
  useEffect(() => {
    if (!exportOpen) return
    const handler = (e: MouseEvent) => {
      if (exportRef.current && !exportRef.current.contains(e.target as Node)) {
        setExportOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [exportOpen])

  const handleExportDoc = async (format: 'markdown' | 'html') => {
    setExporting(true)
    setExportOpen(false)
    try {
      const blob = await api.docs.exportDoc(doc.path, format)
      const ext = format === 'markdown' ? 'md' : 'html'
      const filename = doc.path.replace(/\.md$/, '') + '.' + ext
      triggerDownload(blob, filename.split('/').pop() ?? filename)
      toast(`Exported as ${format.toUpperCase()}`, 'success')
    } catch (err: unknown) {
      toast(err instanceof Error ? err.message : 'Export failed', 'error')
    } finally {
      setExporting(false)
    }
  }

  const title = doc.title ?? titleFromPath(doc.path)
  const renderedContent = useMemo(() => markdownToHtml(doc.content), [doc.content])

  // Load star status + doc status
  useEffect(() => {
    if (!doc.path) return
    api.stars.check(doc.path).then((r) => setStarred(r.starred)).catch(() => {})
    api.docs.status(doc.path).then(setStatus).catch(() => {})
    // Record view
    api.views.record(doc.path).catch(() => {})
  }, [doc.path])

  const handleStar = async () => {
    setStarLoading(true)
    try {
      const r = await api.stars.toggle(doc.path)
      setStarred(r.starred)
    } catch { /* ignore */ }
    finally { setStarLoading(false) }
  }

  const handlePublish = async () => {
    if (!status) return
    setPublishLoading(true)
    try {
      if (status.published) {
        const updated = await api.docs.unpublish(doc.path)
        setStatus(updated)
      } else {
        const updated = await api.docs.publish(doc.path)
        setStatus(updated)
      }
    } catch { /* ignore */ }
    finally { setPublishLoading(false) }
  }

  return (
    <div className={cn('mx-auto max-w-3xl px-8 py-12', className)}>
      {/* Header */}
      <header className="mb-8">
        <div className="mb-3 flex items-center gap-2">
          {status && (
            <span className={cn(
              'flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium',
              status.published
                ? 'bg-green-900/40 text-green-400'
                : 'bg-yellow-900/40 text-yellow-500',
            )}>
              <Circle className="h-1.5 w-1.5 fill-current" />
              {status.published ? 'Published' : 'Draft'}
            </span>
          )}
          {status?.is_pinned && <span title="Pinned">📌</span>}
        </div>

        <h1 className="text-3xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">{title}</h1>

        <div className="mt-3 flex flex-wrap items-center gap-4 text-xs text-zinc-500">
          {doc.updatedAt && (
            <span className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {new Date(doc.updatedAt).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' })}
            </span>
          )}
          {doc.commit_sha && (
            <span className="flex items-center gap-1">
              <GitCommit className="h-3 w-3" />
              {doc.commit_sha.slice(0, 7)}
            </span>
          )}
        </div>
      </header>

      {/* Content */}
      <article
        className="prose prose-zinc dark:prose-invert max-w-none
          prose-headings:font-semibold prose-headings:tracking-tight
          prose-h2:mt-10 prose-h2:text-xl
          prose-h3:mt-8 prose-h3:text-lg
          prose-p:leading-7 prose-p:text-zinc-300
          prose-a:text-blue-400 prose-a:no-underline hover:prose-a:underline
          prose-code:rounded prose-code:bg-zinc-800 prose-code:px-1.5 prose-code:py-0.5 prose-code:text-sm prose-code:text-zinc-300
          prose-pre:bg-zinc-900 prose-pre:border prose-pre:border-zinc-800
          prose-li:text-zinc-300
          prose-strong:text-zinc-200"
        dangerouslySetInnerHTML={{ __html: renderedContent }}
      />

      {/* Action bar */}
      <div className="mt-12 flex flex-wrap items-center gap-2 border-t border-zinc-200 pt-6 dark:border-zinc-800">
        {onEdit && (
          <button
            onClick={onEdit}
            className="flex items-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-300"
          >
            <Edit3 className="h-3 w-3" />
            Edit
          </button>
        )}

        <button
          onClick={handleStar}
          disabled={starLoading}
          className={cn(
            'flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs transition-colors',
            starred
              ? 'border-amber-800/50 bg-amber-900/20 text-amber-400 hover:bg-amber-900/30'
              : 'border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-zinc-700 hover:text-zinc-300',
          )}
        >
          <Star className={cn('h-3 w-3', starred && 'fill-amber-400')} />
          {starred ? 'Starred' : 'Star'}
        </button>

        <button
          onClick={() => setShareOpen(true)}
          className="flex items-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-300"
        >
          <Share2 className="h-3 w-3" />
          Share
        </button>

        {isEditorOrAdmin && status && (
          <button
            onClick={handlePublish}
            disabled={publishLoading}
            className={cn(
              'flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
              status.published
                ? 'border-zinc-800 bg-zinc-900 text-zinc-400 hover:border-red-800 hover:text-red-400'
                : 'border-blue-800/50 bg-blue-900/20 text-blue-400 hover:bg-blue-900/30',
            )}
          >
            <Globe className="h-3 w-3" />
            {publishLoading ? '...' : status.published ? 'Unpublish' : 'Publish'}
          </button>
        )}

        {/* Export dropdown */}
        <div ref={exportRef} className="relative">
          <button
            onClick={() => setExportOpen((v) => !v)}
            disabled={exporting}
            className="flex items-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-300"
          >
            <Download className="h-3 w-3" />
            Export
            <ChevronDown className="h-3 w-3" />
          </button>
          {exportOpen && (
            <div className="absolute right-0 top-full z-20 mt-1 w-52 rounded-lg border border-zinc-800 bg-zinc-900 py-1 shadow-xl">
              <button
                onClick={() => handleExportDoc('markdown')}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
              >
                <FileText className="h-4 w-4" />
                Export as Markdown
              </button>
              <button
                onClick={() => handleExportDoc('html')}
                className="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors"
              >
                <Globe className="h-4 w-4" />
                Export as HTML
              </button>
            </div>
          )}
        </div>

        <div className="flex-1" />

        <button
          onClick={onToggleComments}
          className="flex items-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-300"
        >
          <MessageSquare className="h-3 w-3" />
          Comments
        </button>
        <button
          onClick={onToggleHistory}
          className="flex items-center gap-1.5 rounded-md border border-zinc-800 bg-zinc-900 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-300"
        >
          <History className="h-3 w-3" />
          History
        </button>
      </div>

      {shareOpen && <ShareDialog docPath={doc.path} onClose={() => setShareOpen(false)} />}
    </div>
  )
}

function titleFromPath(path: string): string {
  const name = path.split('/').pop() ?? path
  return name.replace(/\.md$/, '').replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}

function markdownToHtml(md: string): string {
  let html = md
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/\n\n/g, '</p><p>')
    .replace(/^- (.+)$/gm, '<li>$1</li>')

  if (!html.startsWith('<')) html = `<p>${html}</p>`
  html = html.replace(/(<li>.*?<\/li>\n?)+/g, '<ul>$&</ul>')
  return html
}
