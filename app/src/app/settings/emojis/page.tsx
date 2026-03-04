'use client'

import { useState, useEffect, useRef } from 'react'
import { api } from '@/lib/api'
import type { CustomEmoji } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useToast } from '@/contexts/toast-context'
import { Smile, Plus, Trash2, Loader2, Upload } from 'lucide-react'

export default function EmojisPage() {
  const { toast } = useToast()
  const [emojis, setEmojis] = useState<CustomEmoji[]>([])
  const [loading, setLoading] = useState(true)
  const [showAdd, setShowAdd] = useState(false)

  const load = () => {
    api.emojis
      .list()
      .then(setEmojis)
      .catch(() => {})
      .finally(() => setLoading(false))
  }

  useEffect(() => { load() }, [])

  const handleDelete = async (id: string, shortcode: string) => {
    try {
      await api.emojis.delete(id)
      setEmojis((prev) => prev.filter((e) => e.id !== id))
      toast(`Emoji :${shortcode}: deleted`, 'success')
    } catch (err: unknown) {
      toast(err instanceof Error ? err.message : 'Failed to delete emoji', 'error')
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
      </div>
    )
  }

  return (
    <div className="max-w-2xl px-8 py-10">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="mb-1 text-xl font-semibold text-zinc-100">Custom Emoji</h1>
          <p className="text-sm text-zinc-500">Upload custom emoji for your team to use in documents</p>
        </div>
        <button
          onClick={() => setShowAdd(true)}
          className="flex items-center gap-2 rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 transition-colors"
        >
          <Plus className="h-4 w-4" />
          Add emoji
        </button>
      </div>

      {showAdd && (
        <AddEmojiForm
          onClose={() => setShowAdd(false)}
          onAdded={(emoji) => {
            setEmojis((prev) => [emoji, ...prev])
            setShowAdd(false)
            toast(`Emoji :${emoji.shortcode}: added`, 'success')
          }}
          toast={toast}
        />
      )}

      {emojis.length === 0 ? (
        <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed border-zinc-800 py-16 text-center">
          <Smile className="h-10 w-10 text-zinc-700" />
          <p className="text-sm font-medium text-zinc-500">No custom emoji yet</p>
          <p className="text-xs text-zinc-600">Upload PNG or GIF images to use as emoji in your docs</p>
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {emojis.map((emoji) => (
            <EmojiCard key={emoji.id} emoji={emoji} onDelete={handleDelete} />
          ))}
        </div>
      )}
    </div>
  )
}

function EmojiCard({
  emoji,
  onDelete,
}: {
  emoji: CustomEmoji
  onDelete: (id: string, shortcode: string) => void
}) {
  const [confirmDelete, setConfirmDelete] = useState(false)

  return (
    <div className="group relative flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/50 p-4">
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src={emoji.url} alt={emoji.name} className="h-10 w-10 object-contain" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-zinc-200">{emoji.name}</p>
        <p className="truncate text-xs text-zinc-500">:{emoji.shortcode}:</p>
      </div>
      <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
        {confirmDelete ? (
          <>
            <button
              onClick={() => onDelete(emoji.id, emoji.shortcode)}
              className="rounded px-2 py-1 text-xs font-medium text-red-400 hover:bg-red-900/20"
            >
              Delete
            </button>
            <button
              onClick={() => setConfirmDelete(false)}
              className="rounded px-2 py-1 text-xs text-zinc-500 hover:bg-zinc-800"
            >
              Cancel
            </button>
          </>
        ) : (
          <button
            onClick={() => setConfirmDelete(true)}
            className="rounded p-1 text-zinc-600 hover:bg-zinc-800 hover:text-red-400"
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        )}
      </div>
    </div>
  )
}

function AddEmojiForm({
  onClose,
  onAdded,
  toast,
}: {
  onClose: () => void
  onAdded: (emoji: CustomEmoji) => void
  toast: (msg: string, type?: 'success' | 'error' | 'info' | 'warning') => void
}) {
  const [name, setName] = useState('')
  const [shortcode, setShortcode] = useState('')
  const [url, setUrl] = useState('')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Auto-generate shortcode from name
  const handleNameChange = (v: string) => {
    setName(v)
    if (!shortcode) {
      setShortcode(v.toLowerCase().replace(/[^a-z0-9]/g, '_').replace(/__+/g, '_').replace(/^_|_$/g, ''))
    }
  }

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    const reader = new FileReader()
    reader.onload = (ev) => setUrl(ev.target?.result as string)
    reader.readAsDataURL(file)
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!name.trim() || !shortcode.trim() || !url) return
    setSaving(true)
    setError(null)
    try {
      const emoji = await api.emojis.create({ name, shortcode, url })
      onAdded(emoji)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to add emoji')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="mb-6 rounded-xl border border-zinc-700 bg-zinc-900 p-5">
      <h2 className="mb-4 text-sm font-medium text-zinc-200">Add custom emoji</h2>
      <form onSubmit={handleSave} className="space-y-4">
        {/* File upload / URL */}
        <div>
          <label className="mb-1.5 block text-xs font-medium text-zinc-400">Image</label>
          <div className="flex gap-2">
            <input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="Paste image URL or upload a file"
              className="flex-1 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
            />
            <button
              type="button"
              onClick={() => fileInputRef.current?.click()}
              className="flex items-center gap-1.5 rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2 text-sm text-zinc-400 hover:border-zinc-600 hover:text-zinc-200 transition-colors"
            >
              <Upload className="h-4 w-4" />
              Upload
            </button>
            <input
              ref={fileInputRef}
              type="file"
              accept="image/png,image/gif,image/webp"
              className="hidden"
              onChange={handleFileChange}
            />
          </div>
          {url && (
            <div className="mt-2 flex items-center gap-2">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img src={url} alt="preview" className="h-10 w-10 rounded object-contain border border-zinc-800" />
              <span className="text-xs text-zinc-500">Preview</span>
            </div>
          )}
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Name</label>
            <input
              value={name}
              onChange={(e) => handleNameChange(e.target.value)}
              placeholder="Thumbs up"
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Shortcode</label>
            <div className="flex items-center rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5">
              <span className="text-zinc-600">:</span>
              <input
                value={shortcode}
                onChange={(e) => setShortcode(e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, ''))}
                placeholder="thumbs_up"
                className="flex-1 bg-transparent text-sm text-zinc-100 placeholder:text-zinc-600 focus:outline-none"
              />
              <span className="text-zinc-600">:</span>
            </div>
          </div>
        </div>

        {error && <p className="text-xs text-red-400">{error}</p>}

        <div className="flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-1.5 text-sm text-zinc-500 hover:text-zinc-300"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={saving || !name.trim() || !shortcode.trim() || !url}
            className={cn(
              'flex items-center gap-2 rounded-lg px-4 py-1.5 text-sm font-medium transition-colors',
              saving || !name.trim() || !shortcode.trim() || !url
                ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : null}
            {saving ? 'Adding...' : 'Add emoji'}
          </button>
        </div>
      </form>
    </div>
  )
}
