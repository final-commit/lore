'use client'

import { useState, useEffect, useRef } from 'react'
import { Search, X } from 'lucide-react'
import { api } from '@/lib/api'
import type { CustomEmoji } from '@/lib/api'
import { cn } from '@/lib/utils'

// Common emoji categories
const COMMON_EMOJIS = [
  // Smileys
  '😀','😂','😍','🤔','😎','😭','🥳','😴','🤩','😤',
  // Gestures
  '👍','👎','👏','🙌','🤝','✌️','🤞','💪','🙏','👀',
  // Nature
  '🌟','⭐','🔥','💧','🌊','🌈','☀️','🌙','❄️','🌿',
  // Objects
  '💡','🔑','🔒','📌','📎','✏️','📝','📚','🔍','🎯',
  // Symbols
  '✅','❌','⚠️','💬','💭','🔔','📣','🏷️','🎁','🎉',
  // Tech
  '💻','📱','🖥️','⌨️','🖱️','📷','🎧','🔌','💾','🖨️',
]

interface EmojiPickerProps {
  onSelect: (emoji: string) => void
  onClose: () => void
}

export function EmojiPicker({ onSelect, onClose }: EmojiPickerProps) {
  const [search, setSearch] = useState('')
  const [customEmojis, setCustomEmojis] = useState<CustomEmoji[]>([])
  const [activeTab, setActiveTab] = useState<'common' | 'custom'>('common')
  const pickerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    api.emojis.list().then(setCustomEmojis).catch(() => {})
    setTimeout(() => inputRef.current?.focus(), 50)
  }, [])

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [onClose])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onClose])

  const filteredCommon = search
    ? COMMON_EMOJIS.filter((e) => e.includes(search))
    : COMMON_EMOJIS

  const filteredCustom = customEmojis.filter(
    (e) =>
      !search ||
      e.name.toLowerCase().includes(search.toLowerCase()) ||
      e.shortcode.toLowerCase().includes(search.toLowerCase()),
  )

  return (
    <div
      ref={pickerRef}
      className="absolute bottom-full left-0 z-50 mb-1 w-64 rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl"
    >
      {/* Search */}
      <div className="flex items-center gap-2 border-b border-zinc-800 px-3 py-2">
        <Search className="h-3.5 w-3.5 shrink-0 text-zinc-600" />
        <input
          ref={inputRef}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search emoji..."
          className="flex-1 bg-transparent text-sm text-zinc-200 placeholder:text-zinc-600 focus:outline-none"
        />
        <button onClick={onClose} className="text-zinc-600 hover:text-zinc-400">
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {/* Tabs (show only if custom emojis exist) */}
      {customEmojis.length > 0 && (
        <div className="flex border-b border-zinc-800">
          <button
            onClick={() => setActiveTab('common')}
            className={cn(
              'flex-1 py-1.5 text-xs font-medium transition-colors',
              activeTab === 'common' ? 'text-zinc-200' : 'text-zinc-500 hover:text-zinc-300',
            )}
          >
            Common
          </button>
          <button
            onClick={() => setActiveTab('custom')}
            className={cn(
              'flex-1 py-1.5 text-xs font-medium transition-colors',
              activeTab === 'custom' ? 'text-zinc-200' : 'text-zinc-500 hover:text-zinc-300',
            )}
          >
            Custom ({customEmojis.length})
          </button>
        </div>
      )}

      {/* Emoji grid */}
      <div className="max-h-48 overflow-y-auto p-2">
        {activeTab === 'common' && (
          <>
            {filteredCommon.length === 0 ? (
              <p className="py-4 text-center text-xs text-zinc-600">No matching emoji</p>
            ) : (
              <div className="grid grid-cols-8 gap-0.5">
                {filteredCommon.map((emoji) => (
                  <button
                    key={emoji}
                    onClick={() => { onSelect(emoji); onClose() }}
                    title={emoji}
                    className="flex h-8 w-8 items-center justify-center rounded text-lg hover:bg-zinc-800 transition-colors"
                  >
                    {emoji}
                  </button>
                ))}
              </div>
            )}
          </>
        )}

        {activeTab === 'custom' && (
          <>
            {filteredCustom.length === 0 ? (
              <p className="py-4 text-center text-xs text-zinc-600">No custom emoji</p>
            ) : (
              <div className="grid grid-cols-6 gap-1">
                {filteredCustom.map((emoji) => (
                  <button
                    key={emoji.id}
                    onClick={() => { onSelect(`:${emoji.shortcode}:`); onClose() }}
                    title={`:${emoji.shortcode}:`}
                    className="flex h-10 w-10 items-center justify-center rounded hover:bg-zinc-800 transition-colors"
                  >
                    {/* eslint-disable-next-line @next/next/no-img-element */}
                    <img src={emoji.url} alt={emoji.name} className="h-7 w-7 object-contain" />
                  </button>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}
