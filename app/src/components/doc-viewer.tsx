'use client'

import { useState, useEffect, useRef } from 'react'
import { Clock, GitCommit, Edit3, MessageSquare, History, Star, Share2, Globe, Circle, Download, ChevronDown, FileText, BookOpen } from 'lucide-react'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import Link from '@tiptap/extension-link'
import Image from '@tiptap/extension-image'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight'
import { Table } from '@tiptap/extension-table'
import { TableRow } from '@tiptap/extension-table-row'
import { TableCell } from '@tiptap/extension-table-cell'
import { TableHeader } from '@tiptap/extension-table-header'
import { common, createLowlight } from 'lowlight'
import { cn } from '@/lib/utils'
import { api, triggerDownload } from '@/lib/api'
import type { DocResponse, DocStatus } from '@/lib/api'
import { useAuth } from '@/contexts/auth-context'
import { ShareDialog } from './sharing/share-dialog'
import { useToast } from '@/contexts/toast-context'

const lowlight = createLowlight(common)

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .trim()
}

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
  const [scrollProgress, setScrollProgress] = useState(0)
  const exportRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const articleRef = useRef<HTMLDivElement>(null)

  const isEditorOrAdmin = user && (user.role === 'editor' || user.role === 'admin')

  // Tiptap read-only editor
  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        codeBlock: false,
      }),
      Markdown,
      Link.configure({
        openOnClick: true,
        autolink: true,
      }),
      Image,
      TaskList,
      TaskItem.configure({
        nested: true,
      }),
      CodeBlockLowlight.configure({
        lowlight,
      }),
      Table.configure({
        resizable: false,
      }),
      TableRow,
      TableCell,
      TableHeader,
    ],
    content: doc.content,
    editable: false,
    immediatelyRender: false,
    editorProps: {
      attributes: {
        class: cn(
          'prose prose-invert prose-zinc max-w-none focus:outline-none px-8 py-6',
          'prose-headings:font-semibold prose-headings:tracking-tight',
          'prose-h1:text-2xl prose-h2:text-xl prose-h3:text-lg',
          'prose-p:leading-7 prose-p:text-zinc-300',
          'prose-a:text-blue-400 prose-a:no-underline hover:prose-a:underline',
          'prose-code:rounded prose-code:bg-zinc-800 prose-code:px-1.5 prose-code:py-0.5 prose-code:text-sm prose-code:text-zinc-300',
          'prose-pre:bg-zinc-900 prose-pre:border prose-pre:border-zinc-800',
          'prose-li:text-zinc-300 prose-strong:text-zinc-200',
        ),
      },
    },
  })

  // Add heading IDs and anchor links after editor renders
  useEffect(() => {
    if (!editor || !articleRef.current) return

    // Small delay to ensure Tiptap has rendered
    const timer = setTimeout(() => {
      const headings = articleRef.current?.querySelectorAll<HTMLElement>('h1, h2, h3')
      headings?.forEach((heading) => {
        const text = heading.textContent ?? ''
        const slug = slugify(text)
        heading.id = slug

        // Add anchor link if not already present
        if (!heading.querySelector('.heading-anchor')) {
          const anchor = document.createElement('a')
          anchor.href = `#${slug}`
          anchor.className = 'heading-anchor'
          anchor.textContent = '#'
          anchor.setAttribute('aria-hidden', 'true')
          anchor.addEventListener('click', (e) => {
            e.preventDefault()
            heading.scrollIntoView({ behavior: 'smooth', block: 'start' })
            history.replaceState(null, '', `#${slug}`)
          })
          heading.style.position = 'relative'
          heading.prepend(anchor)
        }
      })
    }, 100)

    return () => clearTimeout(timer)
  }, [editor, doc.content])

  // Scroll progress bar
  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const handleScroll = () => {
      const { scrollTop, scrollHeight, clientHeight } = container
      const maxScroll = scrollHeight - clientHeight
      if (maxScroll > 0) {
        setScrollProgress(Math.min(1, scrollTop / maxScroll))
      }
    }

    container.addEventListener('scroll', handleScroll, { passive: true })
    return () => container.removeEventListener('scroll', handleScroll)
  }, [])

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
  const wordCount = doc.content.split(/\s+/).filter(Boolean).length
  const readingTime = Math.max(1, Math.round(wordCount / 200))

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
    <div ref={containerRef} className={cn('relative', className)}>
      {/* Scroll progress bar */}
      <div className="sticky top-0 z-10 h-0.5 w-full bg-transparent">
        <div
          className="h-full bg-blue-500 transition-[width] duration-150 ease-out"
          style={{ width: `${scrollProgress * 100}%` }}
        />
      </div>

      <div className="mx-auto max-w-3xl px-8 py-12">
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
            <span className="flex items-center gap-1">
              <BookOpen className="h-3 w-3" />
              {readingTime} min read
            </span>
          </div>
        </header>

        {/* Content — Tiptap read-only */}
        <div ref={articleRef}>
          <EditorContent editor={editor} />
        </div>

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
    </div>
  )
}

function titleFromPath(path: string): string {
  const name = path.split('/').pop() ?? path
  return name.replace(/\.md$/, '').replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase())
}
