'use client'

import { useParams } from 'next/navigation'
import { useCallback, useEffect, useState } from 'react'
import { Sidebar } from '@/components/sidebar/sidebar'
import { MobileSidebar } from '@/components/sidebar/mobile-sidebar'
import { DocViewer } from '@/components/doc-viewer'
import { LoreEditor } from '@/components/editor/forge-editor'
import { CommentPanel } from '@/components/comments/comment-panel'
import { HistoryPanel } from '@/components/history/history-panel'
import { api } from '@/lib/api'
import type { DocResponse } from '@/lib/api'

export default function DocsPage() {
  const params = useParams()
  const slugParts = (params.slug as string[] | undefined) ?? []
  const docPath = slugParts.length > 0 ? `${slugParts.join('/')}.md` : 'index.md'

  const [doc, setDoc] = useState<DocResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [editing, setEditing] = useState(false)
  const [commentsOpen, setCommentsOpen] = useState(false)
  const [historyOpen, setHistoryOpen] = useState(false)

  useEffect(() => {
    setLoading(true)
    setError(null)
    setEditing(false)
    setCommentsOpen(false)
    setHistoryOpen(false)
    api.docs
      .get(docPath)
      .then(setDoc)
      .catch((err: unknown) => setError(err instanceof Error ? err.message : 'Not found'))
      .finally(() => setLoading(false))
  }, [docPath])

  const handleSave = useCallback(
    async (markdown: string) => {
      await api.docs.update(docPath, markdown)
      const updated = await api.docs.get(docPath)
      setDoc(updated)
    },
    [docPath],
  )

  return (
    <div className="flex h-screen">
      <Sidebar className="hidden lg:flex" />
      <MobileSidebar />

      <main className="flex flex-1 flex-col overflow-hidden">
        {loading ? (
          <div className="flex flex-1 items-center justify-center">
            <div className="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-300" />
          </div>
        ) : doc ? (
          <div className="flex flex-1 overflow-hidden">
            <div className="flex-1 overflow-y-auto">
              {editing ? (
                <LoreEditor
                  content={doc.content}
                  onSave={handleSave}
                  className="h-full"
                />
              ) : (
                <DocViewer
                  doc={doc}
                  onEdit={() => setEditing(true)}
                  onToggleComments={() => { setCommentsOpen(!commentsOpen); setHistoryOpen(false) }}
                  onToggleHistory={() => { setHistoryOpen(!historyOpen); setCommentsOpen(false) }}
                />
              )}
            </div>

            {commentsOpen && (
              <CommentPanel
                filePath={docPath}
                open={commentsOpen}
                onClose={() => setCommentsOpen(false)}
              />
            )}
            {historyOpen && (
              <HistoryPanel
                filePath={docPath}
                open={historyOpen}
                onClose={() => setHistoryOpen(false)}
              />
            )}
          </div>
        ) : (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 text-zinc-500">
            <p className="text-lg">Document not found</p>
            <p className="text-sm font-mono">{docPath}</p>
            {error && <p className="text-xs text-red-400">{error}</p>}
          </div>
        )}
      </main>
    </div>
  )
}
