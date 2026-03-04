'use client'

import { useParams } from 'next/navigation'
import { useEffect, useState, useCallback } from 'react'
import Link from 'next/link'
import { FileText, ChevronUp, ChevronDown, Trash2, Plus, ArrowLeft } from 'lucide-react'
import { api } from '@/lib/api'
import type { Collection, CollectionDoc } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function CollectionPage() {
  const { id } = useParams<{ id: string }>()
  const [collection, setCollection] = useState<Collection | null>(null)
  const [docs, setDocs] = useState<CollectionDoc[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const [col, colDocs] = await Promise.all([
        api.collections.get(id),
        api.collections.getDocs(id),
      ])
      setCollection(col)
      setDocs(colDocs.sort((a, b) => a.order - b.order))
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to load collection')
    } finally {
      setLoading(false)
    }
  }, [id])

  useEffect(() => { load() }, [load])

  const handleReorder = async (path: string, direction: 'up' | 'down') => {
    const idx = docs.findIndex((d) => d.path === path)
    if (direction === 'up' && idx === 0) return
    if (direction === 'down' && idx === docs.length - 1) return
    const newDocs = [...docs]
    const swapIdx = direction === 'up' ? idx - 1 : idx + 1
    ;[newDocs[idx], newDocs[swapIdx]] = [newDocs[swapIdx], newDocs[idx]]
    setDocs(newDocs)
    try {
      await api.collections.reorderDoc(id, path, swapIdx)
    } catch { /* ignore */ }
  }

  const handleRemove = async (path: string) => {
    try {
      await api.collections.removeDoc(id, path)
      setDocs((prev) => prev.filter((d) => d.path !== path))
    } catch { /* ignore */ }
  }

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
      </div>
    )
  }

  if (error || !collection) {
    return (
      <div className="flex h-screen flex-col items-center justify-center gap-3 text-zinc-500">
        <p>{error ?? 'Collection not found'}</p>
        <Link href="/docs" className="text-sm text-blue-400 hover:text-blue-300">
          ← Back to docs
        </Link>
      </div>
    )
  }

  return (
    <div className="mx-auto max-w-3xl px-8 py-12">
      {/* Back */}
      <Link href="/docs" className="mb-6 flex items-center gap-1.5 text-xs text-zinc-500 hover:text-zinc-300 transition-colors">
        <ArrowLeft className="h-3.5 w-3.5" />
        Back
      </Link>

      {/* Collection header */}
      <div className="mb-8">
        <div className="flex items-center gap-3">
          <div
            className="flex h-12 w-12 items-center justify-center rounded-xl text-2xl"
            style={{ backgroundColor: collection.color ?? '#3b82f6' + '33' }}
          >
            {collection.emoji ?? '📁'}
          </div>
          <div>
            <h1 className="text-2xl font-bold text-zinc-100">{collection.name}</h1>
            {collection.description && (
              <p className="mt-0.5 text-sm text-zinc-500">{collection.description}</p>
            )}
          </div>
        </div>
        <div className="mt-4 flex items-center gap-6 text-sm text-zinc-600">
          <span>{collection.doc_count} documents</span>
          <span>{collection.member_count} members</span>
        </div>
      </div>

      {/* Doc list */}
      <div className="space-y-1">
        {docs.length === 0 ? (
          <div className="rounded-xl border border-dashed border-zinc-800 px-6 py-12 text-center">
            <FileText className="mx-auto h-8 w-8 text-zinc-700 mb-2" />
            <p className="text-sm text-zinc-500">No documents in this collection</p>
            <p className="mt-1 text-xs text-zinc-600">Add docs from the sidebar by hovering a document</p>
          </div>
        ) : (
          docs.map((doc, i) => (
            <div
              key={doc.path}
              className="group flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-3 hover:bg-zinc-800/50 transition-colors"
            >
              <FileText className="h-4 w-4 shrink-0 text-zinc-500" />
              <Link
                href={`/docs/${doc.path.replace(/\.md$/, '')}`}
                className="flex-1 text-sm font-medium text-zinc-200 hover:text-white transition-colors"
              >
                {doc.title || doc.path}
              </Link>
              <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => handleReorder(doc.path, 'up')}
                  disabled={i === 0}
                  className="rounded p-1 text-zinc-600 hover:bg-zinc-700 hover:text-zinc-400 disabled:opacity-30 transition-colors"
                >
                  <ChevronUp className="h-3.5 w-3.5" />
                </button>
                <button
                  onClick={() => handleReorder(doc.path, 'down')}
                  disabled={i === docs.length - 1}
                  className="rounded p-1 text-zinc-600 hover:bg-zinc-700 hover:text-zinc-400 disabled:opacity-30 transition-colors"
                >
                  <ChevronDown className="h-3.5 w-3.5" />
                </button>
                <button
                  onClick={() => handleRemove(doc.path)}
                  className="rounded p-1 text-zinc-600 hover:bg-red-900/30 hover:text-red-400 transition-colors"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}
