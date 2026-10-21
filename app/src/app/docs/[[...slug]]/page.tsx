'use client'

import { useParams } from 'next/navigation'
import { useCallback, useEffect, useState, useMemo } from 'react'
import { Sidebar } from '@/components/sidebar/sidebar'
import { MobileSidebar } from '@/components/sidebar/mobile-sidebar'
import { DocViewer } from '@/components/doc-viewer'
import { ForgeEditor as LoreEditor } from '@/components/editor/forge-editor'
import { CommentPanel } from '@/components/comments/comment-panel'
import { HistoryPanel } from '@/components/history/history-panel'
import { TableOfContents } from '@/components/toc'
import { api } from '@/lib/api'
import type { DocResponse } from '@/lib/api'

function countHeadings(content: string): number {
  const regex = /^#{1,3}\s+.+$/gm
  let count = 0
  while (regex.exec(content) !== null) count++
  return count
}

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

  const showToc = useMemo(() => {
    return doc && !editing && countHeadings(doc.content) >= 3
  }, [doc, editing])

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
                <div className="flex justify-center">
                  <div className="flex-1 min-w-0">
                    <DocViewer
                      doc={doc}
                      onEdit={() => setEditing(true)}
                      onToggleComments={() => { setCommentsOpen(!commentsOpen); setHistoryOpen(false) }}
                      onToggleHistory={() => { setHistoryOpen(!historyOpen); setCommentsOpen(false) }}
                    />
                  </div>

                  {showToc && (
                    <div className="hidden xl:block w-56 shrink-0 pr-6">
                      <div className="sticky top-8 pt-12">
                        <TableOfContents content={doc.content} />
                      </div>
                    </div>
                  )}
                </div>
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
