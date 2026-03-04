'use client'

import { useState, useEffect } from 'react'
import { X, Link2, Copy, Check, Globe, Lock } from 'lucide-react'
import { cn } from '@/lib/utils'
import { api } from '@/lib/api'
import type { ShareSettings } from '@/lib/api'

interface ShareDialogProps {
  docPath: string
  onClose: () => void
}

export function ShareDialog({ docPath, onClose }: ShareDialogProps) {
  const [settings, setSettings] = useState<ShareSettings | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [copied, setCopied] = useState(false)
  const [includeChildren, setIncludeChildren] = useState(false)

  useEffect(() => {
    api.share.get(docPath)
      .then((s) => {
        setSettings(s)
        setIncludeChildren(s.include_children)
      })
      .catch(() => setSettings({ enabled: false, url_id: null, include_children: false, created_at: null }))
      .finally(() => setLoading(false))
  }, [docPath])

  const shareUrl = settings?.url_id
    ? `${typeof window !== 'undefined' ? window.location.origin : ''}/shared/${settings.url_id}`
    : null

  const handleToggle = async () => {
    if (!settings) return
    setSaving(true)
    try {
      if (!settings.enabled) {
        const updated = await api.share.create(docPath, includeChildren)
        setSettings(updated)
      } else {
        const updated = await api.share.update(docPath, { enabled: false })
        setSettings(updated)
      }
    } catch { /* ignore */ }
    finally { setSaving(false) }
  }

  const handleUpdateChildren = async (include: boolean) => {
    setIncludeChildren(include)
    if (!settings?.enabled) return
    setSaving(true)
    try {
      const updated = await api.share.update(docPath, { include_children: include })
      setSettings(updated)
    } catch { /* ignore */ }
    finally { setSaving(false) }
  }

  const handleCopy = () => {
    if (!shareUrl) return
    navigator.clipboard.writeText(shareUrl)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className="relative w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-900 shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-zinc-800 px-5 py-4">
          <div className="flex items-center gap-2">
            <Link2 className="h-4 w-4 text-zinc-400" />
            <h2 className="text-sm font-semibold text-zinc-200">Share document</h2>
          </div>
          <button onClick={onClose} className="text-zinc-500 hover:text-zinc-300 transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="p-5 space-y-5">
          {loading ? (
            <div className="flex items-center justify-center py-8">
              <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
            </div>
          ) : (
            <>
              {/* Toggle public link */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  {settings?.enabled ? (
                    <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-green-900/40">
                      <Globe className="h-4 w-4 text-green-400" />
                    </div>
                  ) : (
                    <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-zinc-800">
                      <Lock className="h-4 w-4 text-zinc-500" />
                    </div>
                  )}
                  <div>
                    <p className="text-sm font-medium text-zinc-200">
                      {settings?.enabled ? 'Public link enabled' : 'Private'}
                    </p>
                    <p className="text-xs text-zinc-500">
                      {settings?.enabled ? 'Anyone with the link can view' : 'Only team members can view'}
                    </p>
                  </div>
                </div>
                <button
                  onClick={handleToggle}
                  disabled={saving}
                  className={cn(
                    'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
                    settings?.enabled ? 'bg-blue-600' : 'bg-zinc-700',
                  )}
                >
                  <span className={cn(
                    'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform',
                    settings?.enabled ? 'translate-x-6' : 'translate-x-1',
                  )} />
                </button>
              </div>

              {/* Share URL */}
              {settings?.enabled && shareUrl && (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2">
                    <span className="flex-1 truncate text-xs font-mono text-zinc-400">{shareUrl}</span>
                    <button
                      onClick={handleCopy}
                      className={cn(
                        'flex items-center gap-1 rounded px-2 py-1 text-xs transition-colors',
                        copied
                          ? 'text-green-400'
                          : 'text-zinc-500 hover:bg-zinc-700 hover:text-zinc-300',
                      )}
                    >
                      {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                      {copied ? 'Copied!' : 'Copy'}
                    </button>
                  </div>

                  {/* Include children */}
                  <label className="flex items-center gap-2.5 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={includeChildren}
                      onChange={(e) => handleUpdateChildren(e.target.checked)}
                      className="h-4 w-4 rounded border-zinc-600 bg-zinc-700 accent-blue-500"
                    />
                    <span className="text-xs text-zinc-400">Include child documents in shared view</span>
                  </label>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  )
}
