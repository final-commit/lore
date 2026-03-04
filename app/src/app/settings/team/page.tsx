'use client'

import { useState, useEffect } from 'react'
import { api, triggerDownload } from '@/lib/api'
import type { TeamSettings } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useToast } from '@/contexts/toast-context'
import { Download, Loader2 } from 'lucide-react'

const DEFAULT_SETTINGS: TeamSettings = {
  name: 'My Team',
  allow_signup: false,
  default_role: 'viewer',
  require_email_verification: false,
}

export default function TeamSettingsPage() {
  const { toast } = useToast()
  const [settings, setSettings] = useState<TeamSettings>(DEFAULT_SETTINGS)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)
  const [backupLoading, setBackupLoading] = useState(false)

  useEffect(() => {
    api.settings.get()
      .then(setSettings)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  const update = (patch: Partial<TeamSettings>) => setSettings((s) => ({ ...s, ...patch }))

  const handleFullBackup = async () => {
    setBackupLoading(true)
    try {
      // Create export job
      const job = await api.exportJobs.create('full-backup')
      toast('Backup started, preparing download...', 'info')

      // Poll for completion
      let attempts = 0
      const poll = async (): Promise<void> => {
        attempts++
        const status = await api.exportJobs.get(job.id)
        if (status.status === 'done') {
          const blob = await api.exportJobs.download(job.id)
          triggerDownload(blob, `forge-backup-${new Date().toISOString().slice(0, 10)}.zip`)
          toast('Backup downloaded successfully', 'success')
        } else if (status.status === 'failed') {
          throw new Error(status.error ?? 'Backup failed')
        } else if (attempts < 30) {
          // Keep polling every 2s for up to 60s
          await new Promise((r) => setTimeout(r, 2000))
          return poll()
        } else {
          throw new Error('Backup timed out')
        }
      }

      await poll()
    } catch (err: unknown) {
      toast(err instanceof Error ? err.message : 'Backup failed', 'error')
    } finally {
      setBackupLoading(false)
    }
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setSaved(false)
    try {
      const updated = await api.settings.update(settings)
      setSettings(updated)
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch { /* ignore */ }
    finally { setSaving(false) }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
      </div>
    )
  }

  return (
    <div className="max-w-xl px-8 py-10">
      <h1 className="mb-1 text-xl font-semibold text-zinc-100">Team Settings</h1>
      <p className="mb-8 text-sm text-zinc-500">Configure workspace settings for your team</p>

      <form onSubmit={handleSave} className="space-y-6">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Team name</label>
          <input
            value={settings.name}
            onChange={(e) => update({ name: e.target.value })}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 focus:border-blue-500 focus:outline-none"
          />
        </div>

        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Default role for new members</label>
          <select
            value={settings.default_role}
            onChange={(e) => update({ default_role: e.target.value })}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 focus:border-blue-500 focus:outline-none"
          >
            <option value="viewer">Viewer</option>
            <option value="editor">Editor</option>
            <option value="admin">Admin</option>
          </select>
        </div>

        <div className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-4 space-y-4">
          <ToggleSetting
            label="Allow public signup"
            description="Anyone can create an account without an invite"
            checked={settings.allow_signup}
            onChange={(v) => update({ allow_signup: v })}
          />
          <ToggleSetting
            label="Require email verification"
            description="New accounts must verify their email before signing in"
            checked={settings.require_email_verification}
            onChange={(v) => update({ require_email_verification: v })}
          />
        </div>

        <div className="flex items-center gap-3">
          <button
            type="submit"
            disabled={saving}
            className={cn(
              'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
              saving ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed' : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {saving ? 'Saving...' : 'Save settings'}
          </button>
          {saved && <span className="text-xs text-green-400">Saved!</span>}
        </div>
      </form>

      {/* Danger zone */}
      <div className="mt-12 border-t border-zinc-800 pt-8">
        <h2 className="mb-1 text-sm font-semibold text-zinc-300">Data & Backup</h2>
        <p className="mb-4 text-xs text-zinc-500">Export a full backup of all documents and settings</p>
        <button
          onClick={handleFullBackup}
          disabled={backupLoading}
          className={cn(
            'flex items-center gap-2 rounded-lg border px-4 py-2 text-sm font-medium transition-colors',
            backupLoading
              ? 'border-zinc-800 text-zinc-600 cursor-not-allowed'
              : 'border-zinc-700 text-zinc-300 hover:border-zinc-600 hover:bg-zinc-800',
          )}
        >
          {backupLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Download className="h-4 w-4" />
          )}
          {backupLoading ? 'Preparing backup...' : 'Download full backup'}
        </button>
      </div>
    </div>
  )
}

function ToggleSetting({ label, description, checked, onChange }: {
  label: string
  description: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div>
        <p className="text-sm text-zinc-300">{label}</p>
        <p className="text-xs text-zinc-600">{description}</p>
      </div>
      <button
        type="button"
        onClick={() => onChange(!checked)}
        className={cn(
          'mt-0.5 relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors',
          checked ? 'bg-blue-600' : 'bg-zinc-700',
        )}
      >
        <span className={cn(
          'inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform',
          checked ? 'translate-x-4' : 'translate-x-0.5',
        )} />
      </button>
    </div>
  )
}
