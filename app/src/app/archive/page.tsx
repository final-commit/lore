'use client'

import { useEffect, useState, useCallback } from 'react'
import Link from 'next/link'
import { Archive, RotateCcw, ArrowLeft } from 'lucide-react'
import { api } from '@/lib/api'
import type { ArchivedDoc } from '@/lib/api'

export default function ArchivePage() {
  const [docs, setDocs] = useState<ArchivedDoc[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [actionLoading, setActionLoading] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.docs.listArchive()
      setDocs(data)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to load archive')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => { load() }, [load])

  const handleUnarchive = async (path: string) => {
    setActionLoading(path)
    try {
      await api.docs.unarchive(path)
      setDocs((prev) => prev.filter((d) => d.path !== path))
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
        <Archive className="h-6 w-6 text-zinc-500" />
        <div>
          <h1 className="text-2xl font-bold text-zinc-100">Archive</h1>
          <p className="text-sm text-zinc-500">Documents archived for long-term storage</p>
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
          <Archive className="mx-auto h-10 w-10 text-zinc-700 mb-3" />
          <p className="text-sm text-zinc-500">No archived documents</p>
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
                  Archived by {doc.archived_by} · {new Date(doc.archived_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
                </p>
              </div>
              <button
                onClick={() => handleUnarchive(doc.path)}
                disabled={actionLoading === doc.path}
                className="flex items-center gap-1.5 rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-400 opacity-0 group-hover:opacity-100 hover:border-blue-800 hover:text-blue-400 transition-all disabled:opacity-50"
              >
                <RotateCcw className="h-3.5 w-3.5" />
                Unarchive
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
