'use client'

import { useState, useEffect, useCallback } from 'react'
import { Plus, Users, ChevronDown, ChevronRight, Trash2, UserPlus } from 'lucide-react'
import { api } from '@/lib/api'
import type { Group, GroupMember } from '@/lib/api'
import { cn } from '@/lib/utils'

export default function GroupsPage() {
  const [groups, setGroups] = useState<Group[]>([])
  const [loading, setLoading] = useState(true)
  const [expanded, setExpanded] = useState<string | null>(null)
  const [groupMembers, setGroupMembers] = useState<Record<string, GroupMember[]>>({})
  const [newGroupName, setNewGroupName] = useState('')
  const [creating, setCreating] = useState(false)
  const [showCreate, setShowCreate] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.groups.list()
      setGroups(data)
    } catch { setGroups([]) }
    finally { setLoading(false) }
  }, [])

  useEffect(() => { load() }, [load])

  const handleExpand = async (id: string) => {
    if (expanded === id) { setExpanded(null); return }
    setExpanded(id)
    if (!groupMembers[id]) {
      try {
        const members = await api.groups.getMembers(id)
        setGroupMembers((prev) => ({ ...prev, [id]: members }))
      } catch { setGroupMembers((prev) => ({ ...prev, [id]: [] })) }
    }
  }

  const handleCreate = async () => {
    if (!newGroupName.trim()) return
    setCreating(true)
    try {
      const group = await api.groups.create({ name: newGroupName })
      setGroups((prev) => [...prev, group])
      setNewGroupName('')
      setShowCreate(false)
    } catch { /* ignore */ }
    finally { setCreating(false) }
  }

  const handleDelete = async (id: string) => {
    try {
      await api.groups.delete(id)
      setGroups((prev) => prev.filter((g) => g.id !== id))
      if (expanded === id) setExpanded(null)
    } catch { /* ignore */ }
  }

  const handleRemoveMember = async (groupId: string, userId: string) => {
    try {
      await api.groups.removeMember(groupId, userId)
      setGroupMembers((prev) => ({
        ...prev,
        [groupId]: (prev[groupId] ?? []).filter((m) => m.user_id !== userId),
      }))
    } catch { /* ignore */ }
  }

  return (
    <div className="max-w-2xl px-8 py-10">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="mb-1 text-xl font-semibold text-zinc-100">Groups</h1>
          <p className="text-sm text-zinc-500">Organize members into groups for easier permission management</p>
        </div>
        <button
          onClick={() => setShowCreate(true)}
          className="flex items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-500 transition-colors"
        >
          <Plus className="h-4 w-4" />
          New group
        </button>
      </div>

      {showCreate && (
        <div className="mb-4 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4">
          <div className="flex items-center gap-2">
            <input
              value={newGroupName}
              onChange={(e) => setNewGroupName(e.target.value)}
              placeholder="Group name"
              autoFocus
              className="flex-1 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
              onKeyDown={(e) => e.key === 'Enter' && handleCreate()}
            />
            <button
              onClick={handleCreate}
              disabled={creating || !newGroupName.trim()}
              className={cn(
                'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
                creating || !newGroupName.trim()
                  ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-500',
              )}
            >
              {creating ? 'Creating...' : 'Create'}
            </button>
            <button onClick={() => setShowCreate(false)} className="text-zinc-500 hover:text-zinc-300 transition-colors px-2 py-2">
              Cancel
            </button>
          </div>
        </div>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
        </div>
      ) : groups.length === 0 ? (
        <div className="rounded-xl border border-dashed border-zinc-800 px-6 py-12 text-center">
          <Users className="mx-auto h-8 w-8 text-zinc-700 mb-2" />
          <p className="text-sm text-zinc-500">No groups yet</p>
        </div>
      ) : (
        <div className="space-y-1">
          {groups.map((group) => (
            <div key={group.id} className="rounded-xl border border-zinc-800 overflow-hidden">
              <div
                className="flex items-center gap-3 px-4 py-3 cursor-pointer hover:bg-zinc-800/50 transition-colors"
                onClick={() => handleExpand(group.id)}
              >
                {expanded === group.id
                  ? <ChevronDown className="h-4 w-4 text-zinc-500" />
                  : <ChevronRight className="h-4 w-4 text-zinc-500" />}
                <Users className="h-4 w-4 text-zinc-500" />
                <span className="flex-1 text-sm font-medium text-zinc-200">{group.name}</span>
                <span className="text-xs text-zinc-600">{group.member_count} members</span>
                <button
                  onClick={(e) => { e.stopPropagation(); handleDelete(group.id) }}
                  className="rounded p-1 text-zinc-700 hover:text-red-400 hover:bg-red-900/30 transition-colors"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
              {expanded === group.id && (
                <div className="border-t border-zinc-800 bg-zinc-900/30 px-4 py-3">
                  {(groupMembers[group.id] ?? []).length === 0 ? (
                    <p className="text-xs text-zinc-600">No members yet</p>
                  ) : (
                    <ul className="space-y-2">
                      {(groupMembers[group.id] ?? []).map((member) => (
                        <li key={member.user_id} className="flex items-center gap-2">
                          <div className="h-6 w-6 rounded-full bg-zinc-700 flex items-center justify-center text-[10px] font-semibold text-zinc-300">
                            {member.name.charAt(0).toUpperCase()}
                          </div>
                          <span className="flex-1 text-sm text-zinc-400">{member.name}</span>
                          <span className="text-xs text-zinc-600">{member.email}</span>
                          <button
                            onClick={() => handleRemoveMember(group.id, member.user_id)}
                            className="text-zinc-700 hover:text-red-400 transition-colors"
                          >
                            <Trash2 className="h-3 w-3" />
                          </button>
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
