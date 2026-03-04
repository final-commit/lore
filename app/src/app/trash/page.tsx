'use client'

import { useEffect, useState, useCallback } from 'react'
import Link from 'next/link'
import { Trash2, RotateCcw, X, ArrowLeft } from 'lucide-react'
import { api } from '@/lib/api'
import type { TrashedDoc } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function TrashPage() {
  const [docs, setDocs] = useState<TrashedDoc[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.docs.listTrash()
      setDocs(data)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to load trash')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  const handleRestore = async (path: string) => {
    setActionLoading(path)
    try {
      await api.docs.restore(path)
      setDocs((prev) => prev.filter((d) => d.path !== path))
    } catch { /* ignore */ }
    finally { setActionLoading(null) }
  }

  const handlePermanentDelete = async (path: string) => {
    setActionLoading(path)
    try {
      await api.docs.permanentDelete(path)
      setDocs((prev) => prev.filter((d) => d.path !== path))
      setConfirmDelete(null)
    } catch { /* ignore */ }
    finally { setActionLoading(null) }
  }

  return (
    <div className="mx-auto max-w-3xl px-8 py-12">
      <Link href="/docs" className="mb-6 flex items-center gap-1.5 text-xs text-zinc-500 hover:text-zinc-300 transition-colors">
        <ArrowLeft className="h-3.5 w-3.5" />
        Back to docs
      </Link>

      <div className="mb-8 flex items-center gap-3">
        <Trash2 className="h-6 w-6 text-zinc-500" />
        <div>
          <h1 className="text-2xl font-bold text-zinc-100">Trash</h1>
          <p className="text-sm text-zinc-500">Deleted documents can be restored within 30 days</p>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-16">
          <div className="h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
        </div>
      ) : error ? (
        <div className="rounded-xl border border-red-900/50 bg-red-950/20 px-4 py-3 text-sm text-red-400">
          {error}
          <button onClick={load} className="ml-2 underline hover:no-underline">Retry</button>
        </div>
      ) : docs.length === 0 ? (
        <div className="rounded-xl border border-dashed border-zinc-800 px-6 py-16 text-center">
          <Trash2 className="mx-auto h-10 w-10 text-zinc-700 mb-3" />
          <p className="text-sm text-zinc-500">Trash is empty</p>
        </div>
      ) : (
        <div className="space-y-1">
          {docs.map((doc) => (
            <div
              key={doc.path}
              className="group flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-3"
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-zinc-300 truncate">{doc.path}</p>
                <p className="text-xs text-zinc-600">
                  Deleted by {doc.trashed_by} · {formatDate(doc.trashed_at)}
                </p>
              </div>
              <div className="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={() => handleRestore(doc.path)}
                  disabled={actionLoading === doc.path}
                  className="flex items-center gap-1.5 rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-400 hover:border-green-800 hover:text-green-400 transition-colors disabled:opacity-50"
                >
                  <RotateCcw className="h-3.5 w-3.5" />
                  Restore
                </button>
                {confirmDelete === doc.path ? (
                  <div className="flex items-center gap-1">
                    <span className="text-xs text-red-400">Are you sure?</span>
                    <button
                      onClick={() => handlePermanentDelete(doc.path)}
                      disabled={actionLoading === doc.path}
                      className="rounded-lg px-2 py-1 text-xs bg-red-600 text-white hover:bg-red-500 transition-colors"
                    >
                      Delete
                    </button>
                    <button
                      onClick={() => setConfirmDelete(null)}
                      className="rounded-lg px-2 py-1 text-xs text-zinc-500 hover:text-zinc-300"
                    >
                      Cancel
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => setConfirmDelete(doc.path)}
                    className="flex items-center gap-1.5 rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-500 hover:border-red-800 hover:text-red-400 transition-colors"
                  >
                    <X className="h-3.5 w-3.5" />
                    Delete
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}
