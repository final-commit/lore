'use client'

import { useEffect, useState } from 'react'
import { GitCommit, Clock, X, GitCompare } from 'lucide-react'
import { api } from '@/lib/api'
import { cn } from '@/lib/utils'
import type { CommitInfo, DiffResult } from '@/lib/api'

interface HistoryPanelProps {
  filePath: string
  open: boolean
  onClose: () => void
}

export function HistoryPanel({ filePath, open, onClose }: HistoryPanelProps) {
  const [commits, setCommits] = useState<CommitInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [selectedA, setSelectedA] = useState<string | null>(null)
  const [selectedB, setSelectedB] = useState<string | null>(null)
  const [diff, setDiff] = useState<DiffResult | null>(null)
  const [diffLoading, setDiffLoading] = useState(false)
  const [diffMode, setDiffMode] = useState<'unified' | 'split'>('unified')

  useEffect(() => {
    if (!open) return
    setLoading(true)
    setSelectedA(null)
    setSelectedB(null)
    setDiff(null)
    api.docs
      .history(filePath)
      .then(setCommits)
      .catch(() => setCommits([]))
      .finally(() => setLoading(false))
  }, [open, filePath])

  const handleSelectCommit = (sha: string) => {
    if (!selectedA) {
      setSelectedA(sha)
    } else if (!selectedB && sha !== selectedA) {
      setSelectedB(sha)
    } else {
      setSelectedA(sha)
      setSelectedB(null)
      setDiff(null)
    }
  }

  const handleCompare = async () => {
    if (!selectedA || !selectedB) return
    setDiffLoading(true)
    setDiff(null)
    try {
      const result = await api.docs.diff(filePath, selectedA, selectedB)
      setDiff(result)
    } catch { /* ignore */ }
    finally { setDiffLoading(false) }
  }

  if (!open) return null

  return (
    <aside className="flex h-full w-80 flex-col border-l border-zinc-800 bg-zinc-950">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-3">
        <div className="flex items-center gap-2">
          <GitCommit className="h-4 w-4 text-zinc-400" />
          <span className="text-sm font-medium text-zinc-200">History</span>
        </div>
        <div className="flex items-center gap-2">
          {selectedA && selectedB && (
            <button
              onClick={handleCompare}
              disabled={diffLoading}
              className="flex items-center gap-1 rounded px-2 py-1 text-[11px] bg-blue-600 text-white hover:bg-blue-500 transition-colors"
            >
              <GitCompare className="h-3 w-3" />
              Compare
            </button>
          )}
          <button onClick={onClose} className="text-zinc-500 hover:text-zinc-300 transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
      </div>

      {selectedA && (
        <div className="border-b border-zinc-800 px-4 py-2 text-[11px] text-zinc-500">
          {selectedB
            ? <span>Comparing <span className="font-mono text-zinc-300">{selectedA.slice(0, 7)}</span> ↔ <span className="font-mono text-zinc-300">{selectedB.slice(0, 7)}</span></span>
            : <span>Selected <span className="font-mono text-zinc-300">{selectedA.slice(0, 7)}</span> — select another to compare</span>
          }
        </div>
      )}

      {/* Diff view */}
      {diff && (
        <div className="border-b border-zinc-800">
          <div className="flex items-center gap-2 px-4 py-2">
            <button
              onClick={() => setDiffMode('unified')}
              className={cn('text-[11px] px-2 py-0.5 rounded transition-colors', diffMode === 'unified' ? 'bg-zinc-700 text-zinc-200' : 'text-zinc-500 hover:text-zinc-300')}
            >
              Unified
            </button>
            <button
              onClick={() => setDiffMode('split')}
              className={cn('text-[11px] px-2 py-0.5 rounded transition-colors', diffMode === 'split' ? 'bg-zinc-700 text-zinc-200' : 'text-zinc-500 hover:text-zinc-300')}
            >
              Split
            </button>
            <button onClick={() => setDiff(null)} className="ml-auto text-[11px] text-zinc-600 hover:text-zinc-400">
              Close diff
            </button>
          </div>
          <div className="max-h-60 overflow-auto px-2 pb-2">
            <DiffView diff={diff} mode={diffMode} />
          </div>
        </div>
      )}

      {diffLoading && (
        <div className="flex items-center justify-center py-4 border-b border-zinc-800">
          <div className="h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
          <span className="ml-2 text-xs text-zinc-500">Loading diff...</span>
        </div>
      )}

      {/* Commit list */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
          </div>
        ) : commits.length === 0 ? (
          <div className="px-4 py-12 text-center text-sm text-zinc-600">No history available</div>
        ) : (
          <div className="relative">
            <div className="absolute left-6 top-0 bottom-0 w-px bg-zinc-800" />
            {commits.map((commit, i) => {
              const isSelA = selectedA === commit.sha
              const isSelB = selectedB === commit.sha
              return (
                <button
                  key={commit.sha}
                  onClick={() => handleSelectCommit(commit.sha)}
                  className={cn(
                    'relative w-full px-4 py-3 text-left transition-colors',
                    isSelA || isSelB ? 'bg-zinc-800/80' : 'hover:bg-zinc-900/50',
                  )}
                >
                  {/* Timeline dot */}
                  <div className={cn(
                    'absolute left-[21px] top-4 h-2.5 w-2.5 rounded-full border-2',
                    isSelA ? 'border-blue-500 bg-blue-500' :
                    isSelB ? 'border-violet-500 bg-violet-500' :
                    i === 0 ? 'border-zinc-400 bg-zinc-400' : 'border-zinc-700 bg-zinc-800',
                  )} />

                  <div className="ml-8">
                    <p className="text-sm text-zinc-200 leading-snug line-clamp-2">{commit.message}</p>
                    <div className="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-zinc-500">
                      <span className="font-mono">{commit.sha.slice(0, 7)}</span>
                      <span className="flex items-center gap-0.5">
                        <Clock className="h-2.5 w-2.5" />
                        {formatDate(commit.timestamp)}
                      </span>
                      <span>{commit.author}</span>
                    </div>
                  </div>
                </button>
              )
            })}
          </div>
        )}
      </div>
    </aside>
  )
}

// ── Diff view ────────────────────────────────────────────────────────────────

function DiffView({ diff, mode }: { diff: DiffResult; mode: 'unified' | 'split' }) {
  if (mode === 'split') {
    return (
      <div className="grid grid-cols-2 gap-1 font-mono text-[10px]">
        <div className="rounded bg-zinc-900 p-1.5 space-y-0.5">
          {diff.old_content.split('\n').map((line, i) => (
            <div key={i} className="text-zinc-500 px-1">{line || ' '}</div>
          ))}
        </div>
        <div className="rounded bg-zinc-900 p-1.5 space-y-0.5">
          {diff.new_content.split('\n').map((line, i) => (
            <div key={i} className="text-zinc-300 px-1">{line || ' '}</div>
          ))}
        </div>
      </div>
    )
  }

  return (
    <div className="rounded bg-zinc-900 p-1.5 font-mono text-[10px] space-y-0.5">
      {diff.hunks.map((hunk, hi) => (
        <div key={hi}>
          <div className="text-zinc-600 px-1 py-0.5 bg-zinc-800 rounded mb-0.5">
            @@ -{hunk.old_start},{hunk.old_lines} +{hunk.new_start},{hunk.new_lines} @@
          </div>
          {hunk.lines.map((line, li) => (
            <div
              key={li}
              className={cn(
                'px-1 leading-5',
                line.kind === 'added' ? 'bg-green-950/50 text-green-300' :
                line.kind === 'removed' ? 'bg-red-950/50 text-red-300' :
                'text-zinc-500',
              )}
            >
              {line.kind === 'added' ? '+' : line.kind === 'removed' ? '-' : ' '}
              {line.content}
            </div>
          ))}
        </div>
      ))}
    </div>
  )
}

function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  const diffMs = Date.now() - date.getTime()
  const diffMin = Math.floor(diffMs / 60000)
  if (diffMin < 1) return 'just now'
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHr = Math.floor(diffMin / 60)
  if (diffHr < 24) return `${diffHr}h ago`
  const diffDay = Math.floor(diffHr / 24)
  if (diffDay < 30) return `${diffDay}d ago`
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}
