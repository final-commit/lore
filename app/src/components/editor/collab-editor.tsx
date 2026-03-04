'use client'

import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import Collaboration from '@tiptap/extension-collaboration'
import Placeholder from '@tiptap/extension-placeholder'
import Link from '@tiptap/extension-link'
import Image from '@tiptap/extension-image'
import TaskList from '@tiptap/extension-task-list'
import TaskItem from '@tiptap/extension-task-item'
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight'
import { Table } from '@tiptap/extension-table'
import { TableRow } from '@tiptap/extension-table-row'
import { TableCell } from '@tiptap/extension-table-cell'
import { TableHeader } from '@tiptap/extension-table-header'
import { common, createLowlight } from 'lowlight'
import { useEffect, useMemo, useRef, useState, useCallback } from 'react'
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import { EditorToolbar } from './toolbar'
import { EmbedExtension } from './extensions/embed'
import { AiPanel } from '../ai/ai-panel'
import { cn } from '@/lib/utils'
import { useAuth } from '@/contexts/auth-context'

const lowlight = createLowlight(common)

// Deterministic color from a string
const AVATAR_COLORS = [
  '#3b82f6', '#8b5cf6', '#10b981', '#f59e0b',
  '#ef4444', '#06b6d4', '#f97316', '#84cc16',
]
function colorForName(name: string): string {
  let hash = 0
  for (let i = 0; i < name.length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash)
  return AVATAR_COLORS[Math.abs(hash) % AVATAR_COLORS.length]
}

interface AwarenessUser {
  name: string
  color: string
  clientId: number
}

interface CollabEditorProps {
  filePath: string
  initialContent: string
  onSave: (markdown: string) => Promise<void>
  wsUrl?: string
  className?: string
  onShare?: () => void
  onExport?: () => void
}

export function CollabEditor({
  filePath,
  initialContent,
  onSave,
  wsUrl = 'ws://localhost:3000/yjs',
  className,
  onShare,
  onExport,
}: CollabEditorProps) {
  const { user } = useAuth()
  const [saving, setSaving] = useState(false)
  const [dirty, setDirty] = useState(false)
  const [connected, setConnected] = useState(false)
  const [awarenessUsers, setAwarenessUsers] = useState<AwarenessUser[]>([])
  const [showAI, setShowAI] = useState(false)

  const ydoc = useMemo(() => new Y.Doc(), [])
  const providerRef = useRef<WebsocketProvider | null>(null)

  useEffect(() => {
    const provider = new WebsocketProvider(wsUrl, filePath, ydoc, { connect: true })
    providerRef.current = provider

    provider.on('status', ({ status }: { status: string }) => {
      setConnected(status === 'connected')
    })

    // Set our own awareness state
    const name = user?.name ?? 'Anonymous'
    const color = colorForName(name)
    provider.awareness.setLocalStateField('user', { name, color })

    // Track all awareness states
    const updateUsers = () => {
      const states = provider.awareness.getStates()
      const users: AwarenessUser[] = []
      states.forEach((state, clientId) => {
        if (clientId !== provider.awareness.clientID && state.user) {
          users.push({ name: state.user.name, color: state.user.color, clientId })
        }
      })
      setAwarenessUsers(users)
    }

    provider.awareness.on('change', updateUsers)
    updateUsers()

    return () => {
      provider.awareness.off('change', updateUsers)
      provider.destroy()
    }
  }, [wsUrl, filePath, ydoc, user])

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ codeBlock: false, undoRedo: false }),
      Markdown,
      Collaboration.configure({ document: ydoc }),
      Placeholder.configure({ placeholder: 'Start writing...' }),
      Link.configure({ openOnClick: false, autolink: true }),
      Image,
      TaskList,
      TaskItem.configure({ nested: true }),
      CodeBlockLowlight.configure({ lowlight }),
      Table.configure({ resizable: true }),
      TableRow,
      TableCell,
      TableHeader,
      EmbedExtension,
    ],
    immediatelyRender: false,
    editorProps: {
      attributes: {
        class: cn(
          'prose prose-invert prose-zinc max-w-none focus:outline-none min-h-[400px] px-8 py-6',
          'prose-headings:font-semibold prose-headings:tracking-tight',
          'prose-h1:text-2xl prose-h2:text-xl prose-h3:text-lg',
          'prose-p:leading-7 prose-p:text-zinc-300',
          'prose-a:text-blue-400 prose-a:no-underline hover:prose-a:underline',
          'prose-code:rounded prose-code:bg-zinc-800 prose-code:px-1.5 prose-code:py-0.5 prose-code:text-sm',
          'prose-pre:bg-zinc-900 prose-pre:border prose-pre:border-zinc-800',
          'prose-li:text-zinc-300 prose-strong:text-zinc-200',
        ),
      },
    },
    onUpdate: () => setDirty(true),
  })

  const handleSave = useCallback(async () => {
    if (!editor || !dirty) return
    setSaving(true)
    try {
      const markdown = editor.getMarkdown()
      await onSave(markdown)
      setDirty(false)
    } catch (err) {
      console.error('Save failed:', err)
    } finally {
      setSaving(false)
    }
  }, [editor, dirty, onSave])

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const meta = e.metaKey || e.ctrlKey
      if (meta && e.key === 's') {
        e.preventDefault()
        handleSave()
      }
      if (meta && e.key === 'j') {
        e.preventDefault()
        setShowAI((v) => !v)
      }
      if (meta && e.shiftKey && e.key === 'E') {
        e.preventDefault()
        onExport?.()
      }
      if (meta && e.shiftKey && e.key === 'S') {
        e.preventDefault()
        onShare?.()
      }
      if (meta && e.altKey && e.key === '1') {
        e.preventDefault()
        editor?.chain().focus().toggleHeading({ level: 1 }).run()
      }
      if (meta && e.altKey && e.key === '2') {
        e.preventDefault()
        editor?.chain().focus().toggleHeading({ level: 2 }).run()
      }
      if (meta && e.altKey && e.key === '3') {
        e.preventDefault()
        editor?.chain().focus().toggleHeading({ level: 3 }).run()
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [handleSave, editor, onExport, onShare])

  if (!editor) return null

  return (
    <div className={cn('flex flex-col', className)}>
      <EditorToolbar
        editor={editor}
        saving={saving}
        dirty={dirty}
        onSave={handleSave}
        collaborators={awarenessUsers}
        connected={connected}
        onToggleAI={() => setShowAI((v) => !v)}
        showAI={showAI}
      />

      <div className="relative flex-1 overflow-y-auto">
        <div className="mx-auto max-w-3xl">
          <EditorContent editor={editor} />
        </div>
        {showAI && (
          <AiPanel
            editor={editor}
            docPath={filePath}
            onClose={() => setShowAI(false)}
          />
        )}
      </div>
    </div>
  )
}
