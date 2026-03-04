'use client'

import { useState, useEffect } from 'react'
import { api } from '@/lib/api'
import type { Preferences } from '@/lib/api'
import { cn } from '@/lib/utils'

const DEFAULT_PREFS: Preferences = {
  theme: 'dark',
  language: 'en',
  notifications_email: true,
  notifications_in_app: true,
  editor_font_size: 14,
  editor_spell_check: true,
}

export default function PreferencesPage() {
  const [prefs, setPrefs] = useState<Preferences>(DEFAULT_PREFS)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    api.preferences.get()
      .then(setPrefs)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  const update = (patch: Partial<Preferences>) => setPrefs((p) => ({ ...p, ...patch }))

  const handleSave = async () => {
    setSaving(true)
    setSaved(false)
    try {
      const updated = await api.preferences.update(prefs)
      setPrefs(updated)
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
      <h1 className="mb-1 text-xl font-semibold text-zinc-100">Preferences</h1>
      <p className="mb-8 text-sm text-zinc-500">Customize your Forge experience</p>

      <div className="space-y-6">
        {/* Appearance */}
        <Section title="Appearance">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Theme</label>
            <div className="flex gap-2">
              {(['dark', 'light', 'system'] as const).map((t) => (
                <button
                  key={t}
                  onClick={() => update({ theme: t })}
                  className={cn(
                    'rounded-lg border px-3 py-1.5 text-sm capitalize transition-colors',
                    prefs.theme === t
                      ? 'border-blue-500 bg-blue-900/30 text-blue-300'
                      : 'border-zinc-700 text-zinc-400 hover:border-zinc-600 hover:text-zinc-300',
                  )}
                >
                  {t}
                </button>
              ))}
            </div>
          </div>
        </Section>

        {/* Editor */}
        <Section title="Editor">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Font size</label>
            <div className="flex items-center gap-3">
              <input
                type="range"
                min={12}
                max={20}
                step={1}
                value={prefs.editor_font_size}
                onChange={(e) => update({ editor_font_size: Number(e.target.value) })}
                className="w-32 accent-blue-500"
              />
              <span className="text-sm text-zinc-400">{prefs.editor_font_size}px</span>
            </div>
          </div>
          <Toggle
            label="Spell check"
            checked={prefs.editor_spell_check}
            onChange={(v) => update({ editor_spell_check: v })}
          />
        </Section>

        {/* Notifications */}
        <Section title="Notifications">
          <Toggle
            label="Email notifications"
            description="Receive activity updates via email"
            checked={prefs.notifications_email}
            onChange={(v) => update({ notifications_email: v })}
          />
          <Toggle
            label="In-app notifications"
            description="Show notifications in the sidebar"
            checked={prefs.notifications_in_app}
            onChange={(v) => update({ notifications_in_app: v })}
          />
        </Section>

        <div className="flex items-center gap-3 pt-2">
          <button
            onClick={handleSave}
            disabled={saving}
            className={cn(
              'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
              saving ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed' : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {saving ? 'Saving...' : 'Save preferences'}
          </button>
          {saved && <span className="text-xs text-green-400">Saved!</span>}
        </div>
      </div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h2 className="mb-3 text-sm font-semibold text-zinc-300">{title}</h2>
      <div className="space-y-3 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4">{children}</div>
    </div>
  )
}

function Toggle({ label, description, checked, onChange }: {
  label: string
  description?: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <p className="text-sm text-zinc-300">{label}</p>
        {description && <p className="text-xs text-zinc-600">{description}</p>}
      </div>
      <button
        onClick={() => onChange(!checked)}
        className={cn(
          'relative inline-flex h-5 w-9 items-center rounded-full transition-colors',
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
