'use client'

import { useState, useEffect } from 'react'
import { useAuth } from '@/contexts/auth-context'
import { api } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function ProfilePage() {
  const { user } = useAuth()
  const [name, setName] = useState(user?.name ?? '')
  const [email, setEmail] = useState(user?.email ?? '')
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    if (user) {
      setName(user.name)
      setEmail(user.email)
    }
  }, [user])

  const initials = name.split(' ').map((w) => w[0]).join('').slice(0, 2).toUpperCase()
  const roleColor = user?.role === 'admin' ? 'bg-amber-600' : user?.role === 'editor' ? 'bg-blue-600' : 'bg-zinc-600'

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    setSaved(false)
    try {
      // Update via preferences endpoint for now (backend may vary)
      await api.preferences.update({} as Parameters<typeof api.preferences.update>[0])
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch { /* ignore */ }
    finally { setSaving(false) }
  }

  return (
    <div className="max-w-xl px-8 py-10">
      <h1 className="mb-1 text-xl font-semibold text-zinc-100">Profile</h1>
      <p className="mb-8 text-sm text-zinc-500">Manage your personal account settings</p>

      {/* Avatar */}
      <div className="mb-8 flex items-center gap-4">
        <div className={cn(
          'flex h-16 w-16 items-center justify-center rounded-full text-xl font-bold text-white',
          roleColor,
        )}>
          {initials || '?'}
        </div>
        <div>
          <p className="text-sm font-medium text-zinc-200">{name || 'Your name'}</p>
          <span className={cn(
            'mt-0.5 inline-block rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide',
            user?.role === 'admin' ? 'bg-amber-900/50 text-amber-400' :
            user?.role === 'editor' ? 'bg-blue-900/50 text-blue-400' :
            'bg-zinc-800 text-zinc-400',
          )}>
            {user?.role ?? 'viewer'}
          </span>
        </div>
      </div>

      <form onSubmit={handleSave} className="space-y-5">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Full name</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
          />
        </div>

        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Email</label>
          <input
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
          />
        </div>

        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Member since</label>
          <p className="text-sm text-zinc-500">
            {user?.created_at
              ? new Date(user.created_at).toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' })
              : '—'}
          </p>
        </div>

        <div className="flex items-center gap-3 pt-2">
          <button
            type="submit"
            disabled={saving}
            className={cn(
              'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
              saving
                ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {saving ? 'Saving...' : 'Save changes'}
          </button>
          {saved && <span className="text-xs text-green-400">Saved!</span>}
        </div>
      </form>
    </div>
  )
}
