'use client'

import { useParams } from 'next/navigation'
import { useEffect, useState } from 'react'
import { BookOpen } from 'lucide-react'
import { api } from '@/lib/api'
import type { SharedDoc } from '@/lib/api'

export default function SharedDocPage() {
  const { url_id } = useParams<{ url_id: string }>()
  const [doc, setDoc] = useState<SharedDoc | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    api.share.getPublic(url_id)
      .then(setDoc)
      .catch((err: unknown) => setError(err instanceof Error ? err.message : 'Not found'))
      .finally(() => setLoading(false))
  }, [url_id])

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-white dark:bg-zinc-950">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-zinc-300 border-t-zinc-700 dark:border-zinc-700 dark:border-t-zinc-400" />
      </div>
    )
  }

  if (error || !doc) {
    return (
      <div className="flex min-h-screen flex-col items-center justify-center gap-3 bg-white dark:bg-zinc-950">
        <p className="text-zinc-500">This document is not publicly available</p>
        <p className="text-xs text-zinc-600">{error}</p>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-white dark:bg-zinc-950">
      {/* Header */}
      <header className="border-b border-zinc-200 dark:border-zinc-800 px-8 py-4">
        <div className="mx-auto flex max-w-3xl items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded bg-blue-600">
            <BookOpen className="h-3.5 w-3.5 text-white" />
          </div>
          <span className="text-sm font-semibold text-zinc-700 dark:text-zinc-300">Lore</span>
          <span className="mx-2 text-zinc-300 dark:text-zinc-700">/</span>
          <span className="text-sm text-zinc-500">{doc.title}</span>
        </div>
      </header>

      {/* Content */}
      <main className="mx-auto max-w-3xl px-8 py-12">
        <h1 className="mb-8 text-3xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
          {doc.title}
        </h1>

        <article
          className="prose prose-zinc dark:prose-invert max-w-none
            prose-headings:font-semibold prose-headings:tracking-tight
            prose-p:leading-7
            prose-a:text-blue-500 prose-a:no-underline hover:prose-a:underline
            prose-code:rounded prose-code:bg-zinc-100 prose-code:px-1.5 prose-code:py-0.5 prose-code:text-sm
            dark:prose-code:bg-zinc-800"
          dangerouslySetInnerHTML={{ __html: renderMarkdown(doc.content) }}
        />

        {/* Child docs */}
        {doc.children && doc.children.length > 0 && (
          <div className="mt-12 border-t border-zinc-200 dark:border-zinc-800 pt-8">
            <h2 className="mb-4 text-lg font-semibold text-zinc-800 dark:text-zinc-200">Related documents</h2>
            <div className="space-y-3">
              {doc.children.map((child) => (
                <div key={child.path} className="rounded-lg border border-zinc-200 dark:border-zinc-800 p-4">
                  <h3 className="font-medium text-zinc-800 dark:text-zinc-200">{child.title}</h3>
                  <article
                    className="mt-2 text-sm text-zinc-600 dark:text-zinc-400 line-clamp-3 prose prose-sm max-w-none"
                    dangerouslySetInnerHTML={{ __html: renderMarkdown(child.content.slice(0, 300)) }}
                  />
                </div>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}

function renderMarkdown(md: string): string {
  return md
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/\n\n/g, '</p><p>')
    .replace(/^- (.+)$/gm, '<li>$1</li>')
}
