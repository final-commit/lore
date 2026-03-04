'use client'

import { useState, useEffect, useCallback } from 'react'
import { UserPlus, Search, Shield } from 'lucide-react'
import { api } from '@/lib/api'
import type { FullUserInfo } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useAuth } from '@/contexts/auth-context'

const ROLES = ['viewer', 'editor', 'admin']

export default function MembersPage() {
  const { user: currentUser } = useAuth()
  const [users, setUsers] = useState<FullUserInfo[]>([])
  const [loading, setLoading] = useState(true)
  const [search, setSearch] = useState('')
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteRole, setInviteRole] = useState('viewer')
  const [inviting, setInviting] = useState(false)
  const [inviteError, setInviteError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = search.trim()
        ? await api.users.search(search)
        : await api.users.list()
      setUsers(data)
    } catch { setUsers([]) }
    finally { setLoading(false) }
  }, [search])

  useEffect(() => {
    const t = setTimeout(load, 200)
    return () => clearTimeout(t)
  }, [load])

  const handleRoleChange = async (userId: string, role: string) => {
    try {
      const updated = await api.users.updateRole(userId, role)
      setUsers((prev) => prev.map((u) => u.id === userId ? updated : u))
    } catch { /* ignore */ }
  }

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!inviteEmail.trim()) return
    setInviting(true)
    setInviteError(null)
    try {
      await api.users.invite(inviteEmail, inviteRole)
      setInviteEmail('')
    } catch (err: unknown) {
      setInviteError(err instanceof Error ? err.message : 'Failed to send invite')
    } finally {
      setInviting(false)
    }
  }

  const roleColor = (role: string) => {
    if (role === 'admin') return 'bg-amber-900/50 text-amber-400'
    if (role === 'editor') return 'bg-blue-900/50 text-blue-400'
    return 'bg-zinc-800 text-zinc-500'
  }

  return (
    <div className="max-w-2xl px-8 py-10">
      <h1 className="mb-1 text-xl font-semibold text-zinc-100">Members</h1>
      <p className="mb-8 text-sm text-zinc-500">Manage who has access to your workspace</p>

      {/* Invite */}
      <div className="mb-8 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4">
        <h2 className="mb-3 flex items-center gap-2 text-sm font-medium text-zinc-300">
          <UserPlus className="h-4 w-4" />
          Invite member
        </h2>
        <form onSubmit={handleInvite} className="flex items-center gap-2">
          <input
            type="email"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.target.value)}
            placeholder="colleague@company.com"
            className="flex-1 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
          />
          <select
            value={inviteRole}
            onChange={(e) => setInviteRole(e.target.value)}
            className="rounded-lg border border-zinc-700 bg-zinc-800 px-2 py-2 text-sm text-zinc-300 focus:border-blue-500 focus:outline-none"
          >
            {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
          </select>
          <button
            type="submit"
            disabled={inviting || !inviteEmail.trim()}
            className={cn(
              'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
              inviting || !inviteEmail.trim()
                ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {inviting ? 'Sending...' : 'Invite'}
          </button>
        </form>
        {inviteError && <p className="mt-2 text-xs text-red-400">{inviteError}</p>}
      </div>

      {/* Search */}
      <div className="mb-4 flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2">
        <Search className="h-4 w-4 text-zinc-600" />
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search members..."
          className="flex-1 bg-transparent text-sm text-zinc-300 outline-none placeholder:text-zinc-600"
        />
      </div>

      {/* List */}
      {loading ? (
        <div className="flex items-center justify-center py-8">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
        </div>
      ) : (
        <div className="space-y-1">
          {users.map((u) => {
            const initials = u.name.split(' ').map((w) => w[0]).join('').slice(0, 2).toUpperCase()
            const isSelf = u.id === currentUser?.id
            return (
              <div key={u.id} className="flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900/50 px-4 py-3">
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-zinc-700 text-xs font-semibold text-zinc-300">
                  {initials}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-zinc-200 truncate">
                    {u.name}
                    {isSelf && <span className="ml-1.5 text-[10px] text-zinc-600">(you)</span>}
                  </p>
                  <p className="text-xs text-zinc-600 truncate">{u.email}</p>
                </div>
                <select
                  value={u.role}
                  onChange={(e) => handleRoleChange(u.id, e.target.value)}
                  disabled={isSelf}
                  className={cn(
                    'rounded-lg border border-zinc-700 bg-zinc-800 px-2 py-1 text-xs font-medium transition-colors focus:outline-none',
                    roleColor(u.role),
                    isSelf && 'opacity-50 cursor-not-allowed',
                  )}
                >
                  {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
                </select>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
