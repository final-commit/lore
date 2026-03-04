'use client'

import { useState, useEffect, useCallback } from 'react'
import { Plus, Key, Copy, Check, Trash2, Eye, EyeOff } from 'lucide-react'
import { api } from '@/lib/api'
import type { ApiToken } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function ApiTokensPage() {
  const [tokens, setTokens] = useState<ApiToken[]>([])
  const [loading, setLoading] = useState(true)
  const [showCreate, setShowCreate] = useState(false)
  const [newName, setNewName] = useState('')
  const [creating, setCreating] = useState(false)
  const [newToken, setNewToken] = useState<ApiToken | null>(null)
  const [copied, setCopied] = useState(false)
  const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.tokens.list()
      setTokens(data)
    } catch { setTokens([]) }
    finally { setLoading(false) }
  }, [])

  useEffect(() => { load() }, [load])

  const handleCreate = async () => {
    if (!newName.trim()) return
    setCreating(true)
    try {
      const token = await api.tokens.create(newName)
      setNewToken(token)
      setTokens((prev) => [...prev, token])
      setNewName('')
      setShowCreate(false)
    } catch { /* ignore */ }
    finally { setCreating(false) }
  }

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleDelete = async (id: string) => {
    try {
      await api.tokens.delete(id)
      setTokens((prev) => prev.filter((t) => t.id !== id))
      setDeleteConfirm(null)
      if (newToken?.id === id) setNewToken(null)
    } catch { /* ignore */ }
  }

  return (
    <div className="max-w-2xl px-8 py-10">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="mb-1 text-xl font-semibold text-zinc-100">API Tokens</h1>
          <p className="text-sm text-zinc-500">Create tokens for programmatic access to Forge</p>
        </div>
        <button
          onClick={() => setShowCreate(true)}
          className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-500 transition-colors"
        >
          <Plus className="h-4 w-4" />
          New token
        </button>
      </div>

      {/* New token created banner */}
      {newToken?.token && (
        <div className="mb-6 rounded-xl border border-green-900/50 bg-green-950/20 p-4">
          <p className="mb-2 text-sm font-medium text-green-300">Token created — copy it now!</p>
          <p className="mb-2 text-xs text-green-500">This is the only time the token will be shown.</p>
          <div className="flex items-center gap-2 rounded-lg border border-green-900/50 bg-green-950/30 px-3 py-2">
            <code className="flex-1 truncate font-mono text-xs text-green-300">{newToken.token}</code>
            <button
              onClick={() => handleCopy(newToken.token!)}
              className={cn('flex items-center gap-1 text-xs transition-colors', copied ? 'text-green-400' : 'text-green-600 hover:text-green-400')}
            >
              {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              {copied ? 'Copied!' : 'Copy'}
            </button>
          </div>
        </div>
      )}

      {showCreate && (
        <div className="mb-6 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4">
          <p className="mb-3 text-sm font-medium text-zinc-300">Create new token</p>
          <div className="flex items-center gap-2">
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder="e.g. CI/CD pipeline, Local dev"
              autoFocus
              className="flex-1 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
              onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            />
            <button
              onClick={handleCreate}
              disabled={creating || !newName.trim()}
              className={cn(
                'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
                creating || !newName.trim()
                  ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-500',
              )}
            >
              {creating ? 'Creating...' : 'Create'}
            </button>
            <button onClick={() => setShowCreate(false)} className="px-2 py-2 text-zinc-500 hover:text-zinc-300 transition-colors">
              Cancel
            </button>
          </div>
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
        </div>
      ) : tokens.length === 0 ? (
        <div className="rounded-xl border border-dashed border-zinc-800 px-6 py-12 text-center">
          <Key className="mx-auto h-8 w-8 text-zinc-700 mb-2" />
          <p className="text-sm text-zinc-500">No API tokens yet</p>
        </div>
      ) : (
        <div className="space-y-2">
          {tokens.map((token) => (
            <div key={token.id} className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/50 px-4 py-3">
              <Key className="h-4 w-4 shrink-0 text-zinc-500" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-zinc-200">{token.name}</p>
                <div className="flex items-center gap-3 text-xs text-zinc-600">
                  <span className="font-mono">{token.prefix}••••••••</span>
                  <span>Created {new Date(token.created_at).toLocaleDateString()}</span>
                  {token.last_used && <span>Last used {new Date(token.last_used).toLocaleDateString()}</span>}
                </div>
              </div>
              {deleteConfirm === token.id ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-red-400">Revoke?</span>
                  <button
                    onClick={() => handleDelete(token.id)}
                    className="rounded px-2 py-1 text-xs bg-red-600 text-white hover:bg-red-500 transition-colors"
                  >
                    Yes
                  </button>
                  <button
                    onClick={() => setDeleteConfirm(null)}
                    className="text-zinc-500 hover:text-zinc-300 text-xs"
                  >
                    No
                  </button>
                </div>
              ) : (
                <button
                  onClick={() => setDeleteConfirm(token.id)}
                  className="text-zinc-700 hover:text-red-400 hover:bg-red-900/30 rounded p-1 transition-colors"
                  title="Revoke token"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
