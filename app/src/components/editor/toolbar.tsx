'use client'

import type { Editor } from '@tiptap/react'
import {
  Bold, Italic, Strikethrough, Code, Heading1, Heading2, Heading3,
  List, ListOrdered, ListTodo, Quote, Minus, Link as LinkIcon,
  Image as ImageIcon, Undo, Redo, Save, Loader2, Wifi, WifiOff,
  Smile, Sparkles, Globe,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useCallback, useState } from 'react'
import { EmojiPicker } from './emoji-picker'

interface CollaboratorInfo {
  name: string
  color: string
  clientId: number
}

interface EditorToolbarProps {
  editor: Editor
  saving: boolean
  dirty: boolean
  onSave: () => void
  collaborators?: CollaboratorInfo[]
  connected?: boolean
  onToggleAI?: () => void
  showAI?: boolean
}

export function EditorToolbar({
  editor,
  saving,
  dirty,
  onSave,
  collaborators = [],
  connected,
  onToggleAI,
  showAI,
}: EditorToolbarProps) {
  const [showEmojiPicker, setShowEmojiPicker] = useState(false)

  const addLink = useCallback(() => {
    const url = window.prompt('URL')
    if (url) editor.chain().focus().setLink({ href: url }).run()
  }, [editor])

  const addImage = useCallback(() => {
    const url = window.prompt('Image URL')
    if (url) editor.chain().focus().setImage({ src: url }).run()
  }, [editor])

  const addEmbed = useCallback(() => {
    const url = window.prompt('URL to embed')
    if (url) editor.chain().focus().insertEmbed(url).run()
  }, [editor])

  const insertEmoji = useCallback(
    (emoji: string) => {
      editor.chain().focus().insertContent(emoji).run()
    },
    [editor],
  )

  return (
    <div className="sticky top-0 z-10 flex items-center gap-0.5 border-b border-zinc-800 bg-zinc-950/95 px-4 py-1.5 backdrop-blur-sm flex-wrap">
      {/* Text formatting */}
      <ToolbarGroup>
        <ToolbarButton icon={Bold} active={editor.isActive('bold')} onClick={() => editor.chain().focus().toggleBold().run()} tooltip="Bold (⌘B)" />
        <ToolbarButton icon={Italic} active={editor.isActive('italic')} onClick={() => editor.chain().focus().toggleItalic().run()} tooltip="Italic (⌘I)" />
        <ToolbarButton icon={Strikethrough} active={editor.isActive('strike')} onClick={() => editor.chain().focus().toggleStrike().run()} tooltip="Strikethrough" />
        <ToolbarButton icon={Code} active={editor.isActive('code')} onClick={() => editor.chain().focus().toggleCode().run()} tooltip="Inline code (⌘E)" />
      </ToolbarGroup>

      <ToolbarSeparator />

      <ToolbarGroup>
        <ToolbarButton icon={Heading1} active={editor.isActive('heading', { level: 1 })} onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()} tooltip="Heading 1 (⌘⌥1)" />
        <ToolbarButton icon={Heading2} active={editor.isActive('heading', { level: 2 })} onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()} tooltip="Heading 2 (⌘⌥2)" />
        <ToolbarButton icon={Heading3} active={editor.isActive('heading', { level: 3 })} onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()} tooltip="Heading 3 (⌘⌥3)" />
      </ToolbarGroup>

      <ToolbarSeparator />

      <ToolbarGroup>
        <ToolbarButton icon={List} active={editor.isActive('bulletList')} onClick={() => editor.chain().focus().toggleBulletList().run()} tooltip="Bullet list" />
        <ToolbarButton icon={ListOrdered} active={editor.isActive('orderedList')} onClick={() => editor.chain().focus().toggleOrderedList().run()} tooltip="Ordered list" />
        <ToolbarButton icon={ListTodo} active={editor.isActive('taskList')} onClick={() => editor.chain().focus().toggleTaskList().run()} tooltip="Task list" />
      </ToolbarGroup>

      <ToolbarSeparator />

      <ToolbarGroup>
        <ToolbarButton icon={Quote} active={editor.isActive('blockquote')} onClick={() => editor.chain().focus().toggleBlockquote().run()} tooltip="Blockquote" />
        <ToolbarButton icon={Minus} onClick={() => editor.chain().focus().setHorizontalRule().run()} tooltip="Horizontal rule" />
        <ToolbarButton icon={LinkIcon} onClick={addLink} tooltip="Add link" />
        <ToolbarButton icon={ImageIcon} onClick={addImage} tooltip="Add image" />
        <ToolbarButton icon={Globe} onClick={addEmbed} tooltip="Embed URL" />
      </ToolbarGroup>

      <ToolbarSeparator />

      {/* Emoji picker (relative container) */}
      <div className="relative">
        <ToolbarButton
          icon={Smile}
          active={showEmojiPicker}
          onClick={() => setShowEmojiPicker((v) => !v)}
          tooltip="Insert emoji"
        />
        {showEmojiPicker && (
          <EmojiPicker
            onSelect={insertEmoji}
            onClose={() => setShowEmojiPicker(false)}
          />
        )}
      </div>

      <ToolbarSeparator />

      <ToolbarGroup>
        <ToolbarButton icon={Undo} onClick={() => editor.chain().focus().undo().run()} disabled={!editor.can().undo()} tooltip="Undo (⌘Z)" />
        <ToolbarButton icon={Redo} onClick={() => editor.chain().focus().redo().run()} disabled={!editor.can().redo()} tooltip="Redo (⌘⇧Z)" />
      </ToolbarGroup>

      {/* Right side: collaborators + AI + connection + save */}
      <div className="ml-auto flex items-center gap-2">
        {/* Collaborator avatars */}
        {collaborators.length > 0 && (
          <div className="flex items-center">
            {collaborators.slice(0, 5).map((c, i) => {
              const initials = c.name.split(' ').map((w: string) => w[0]).join('').slice(0, 2).toUpperCase()
              return (
                <div
                  key={c.clientId}
                  title={c.name}
                  style={{ backgroundColor: c.color, marginLeft: i === 0 ? 0 : -6 }}
                  className="flex h-6 w-6 items-center justify-center rounded-full border-2 border-zinc-950 text-[9px] font-bold text-white"
                >
                  {initials}
                </div>
              )
            })}
            {collaborators.length > 5 && (
              <div className="-ml-1.5 flex h-6 w-6 items-center justify-center rounded-full border-2 border-zinc-950 bg-zinc-700 text-[9px] font-bold text-zinc-300">
                +{collaborators.length - 5}
              </div>
            )}
          </div>
        )}

        {/* Connection indicator */}
        {connected !== undefined && (
          <div title={connected ? 'Live sync active' : 'Disconnected'}>
            {connected
              ? <Wifi className="h-3.5 w-3.5 text-green-500" />
              : <WifiOff className="h-3.5 w-3.5 text-red-500" />}
          </div>
        )}

        {/* AI toggle */}
        {onToggleAI && (
          <button
            onClick={onToggleAI}
            title="AI Assistant (⌘J)"
            className={cn(
              'flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium transition-colors',
              showAI
                ? 'bg-blue-900/40 text-blue-400'
                : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300',
            )}
          >
            <Sparkles className="h-3.5 w-3.5" />
            <span className="hidden sm:inline">AI</span>
          </button>
        )}

        {dirty && <span className="text-xs text-zinc-500">Unsaved</span>}

        <button
          onClick={onSave}
          disabled={!dirty || saving}
          className={cn(
            'flex items-center gap-1.5 rounded-md px-3 py-1 text-xs font-medium transition-colors',
            dirty ? 'bg-blue-600 text-white hover:bg-blue-500' : 'bg-zinc-800 text-zinc-500',
          )}
        >
          {saving ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />}
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  )
}

function ToolbarButton({
  icon: Icon, active, disabled, onClick, tooltip,
}: {
  icon: React.ComponentType<{ className?: string }>
  active?: boolean
  disabled?: boolean
  onClick: () => void
  tooltip?: string
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      title={tooltip}
      className={cn(
        'rounded p-1.5 transition-colors',
        active ? 'bg-zinc-700 text-zinc-100' : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300',
        disabled && 'cursor-not-allowed opacity-30',
      )}
    >
      <Icon className="h-4 w-4" />
    </button>
  )
}

function ToolbarGroup({ children }: { children: React.ReactNode }) {
  return <div className="flex items-center gap-0.5">{children}</div>
}

function ToolbarSeparator() {
  return <div className="mx-1.5 h-5 w-px bg-zinc-800" />
}
