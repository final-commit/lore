'use client'

import { useState, useEffect } from 'react'
import { Keyboard } from 'lucide-react'
import { api } from '@/lib/api'
import type { Shortcut } from '@/lib/api'

export default function ShortcutsPage() {
  const [shortcuts, setShortcuts] = useState<Shortcut[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.shortcuts.list()
      .then(setShortcuts)
      .catch(() => setShortcuts(FALLBACK_SHORTCUTS))
      .finally(() => setLoading(false))
  }, [])

  const grouped = shortcuts.reduce<Record<string, Shortcut[]>>((acc, s) => {
    if (!acc[s.category]) acc[s.category] = []
    acc[s.category].push(s)
    return acc
  }, {})

  return (
    <div className="max-w-xl px-8 py-10">
      <div className="mb-8 flex items-center gap-3">
        <Keyboard className="h-6 w-6 text-zinc-500" />
        <div>
          <h1 className="text-xl font-semibold text-zinc-100">Keyboard Shortcuts</h1>
          <p className="text-sm text-zinc-500">Keyboard shortcuts to speed up your workflow</p>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-8">
          <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
        </div>
      ) : (
        <div className="space-y-6">
          {Object.entries(grouped).map(([category, items]) => (
            <div key={category}>
              <h2 className="mb-3 text-sm font-semibold text-zinc-400">{category}</h2>
              <div className="rounded-xl border border-zinc-800 overflow-hidden">
                <table className="w-full">
                  <tbody className="divide-y divide-zinc-800">
                    {items.map((shortcut) => (
                      <tr key={shortcut.action} className="flex items-center px-4 py-2.5">
                        <td className="flex-1 text-sm text-zinc-300">{shortcut.description}</td>
                        <td className="flex items-center gap-1">
                          {shortcut.keys.map((key, i) => (
                            <span key={i}>
                              <kbd className="rounded border border-zinc-700 bg-zinc-800 px-1.5 py-0.5 text-[11px] font-mono text-zinc-400">
                                {key}
                              </kbd>
                              {i < shortcut.keys.length - 1 && (
                                <span className="mx-0.5 text-xs text-zinc-600">+</span>
                              )}
                            </span>
                          ))}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

const FALLBACK_SHORTCUTS: Shortcut[] = [
  { action: 'search', description: 'Open search', keys: ['⌘', 'K'], category: 'Navigation' },
  { action: 'new-doc', description: 'New document', keys: ['⌘', 'N'], category: 'Navigation' },
  { action: 'save', description: 'Save document', keys: ['⌘', 'S'], category: 'Editor' },
  { action: 'bold', description: 'Bold text', keys: ['⌘', 'B'], category: 'Editor' },
  { action: 'italic', description: 'Italic text', keys: ['⌘', 'I'], category: 'Editor' },
  { action: 'heading1', description: 'Heading 1', keys: ['⌘', 'Alt', '1'], category: 'Editor' },
  { action: 'heading2', description: 'Heading 2', keys: ['⌘', 'Alt', '2'], category: 'Editor' },
  { action: 'code', description: 'Code block', keys: ['⌘', '`'], category: 'Editor' },
  { action: 'comments', description: 'Toggle comments', keys: ['⌘', 'Shift', 'C'], category: 'View' },
  { action: 'history', description: 'Toggle history', keys: ['⌘', 'Shift', 'H'], category: 'View' },
]
