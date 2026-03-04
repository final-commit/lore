'use client'

import { useState, useRef, useEffect } from 'react'
import type { Editor } from '@tiptap/react'
import {
  X, Sparkles, MessageSquare, Wand2, FileText, Send, Loader2,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { api } from '@/lib/api'

type AiTab = 'ask' | 'improve' | 'summarize' | 'generate'

interface AiPanelProps {
  editor: Editor
  docPath?: string
  onClose: () => void
}

export function AiPanel({ editor, docPath, onClose }: AiPanelProps) {
  const [tab, setTab] = useState<AiTab>('ask')
  const [input, setInput] = useState('')
  const [result, setResult] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [notConfigured, setNotConfigured] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)

  const selectedText = editor.state.selection
    ? editor.state.doc.textBetween(
        editor.state.selection.from,
        editor.state.selection.to,
        ' ',
      )
    : ''

  // Focus input when panel opens
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 50)
  }, [tab])

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onClose])

  const handleAction = async () => {
    setLoading(true)
    setResult(null)
    setError(null)
    setNotConfigured(false)
    try {
      if (tab === 'ask') {
        if (!input.trim()) return
        const res = await api.ai.answer(docPath ?? '', input)
        setResult(res.answer)
      } else if (tab === 'improve') {
        const text = selectedText || input
        if (!text.trim()) { setError('Select some text or type content to improve'); return }
        const suggestions = await api.ai.suggest(docPath ?? '', text)
        if (suggestions.length > 0) {
          setResult(suggestions.map((s) => `**Suggestion:** ${s.suggestion}${s.explanation ? `\n\n*${s.explanation}*` : ''}`).join('\n\n---\n\n'))
        } else {
          setResult('No suggestions — the content looks good!')
        }
      } else if (tab === 'summarize') {
        const content = editor.getText()
        const res = await api.ai.summarize(content)
        setResult(res.summary)
      } else if (tab === 'generate') {
        if (!input.trim()) return
        const res = await api.ai.generate(input)
        // Insert generated content at cursor
        editor.chain().focus().insertContent(res.content).run()
        setResult('Content inserted at cursor.')
        setInput('')
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : 'AI request failed'
      if (msg.includes('501') || msg.toLowerCase().includes('not configured') || msg.toLowerCase().includes('not implemented')) {
        setNotConfigured(true)
      } else {
        setError(msg)
      }
    } finally {
      setLoading(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault()
      handleAction()
    }
  }

  const TABS: { id: AiTab; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
    { id: 'ask', label: 'Ask', icon: MessageSquare },
    { id: 'improve', label: 'Improve', icon: Sparkles },
    { id: 'summarize', label: 'Summarize', icon: FileText },
    { id: 'generate', label: 'Generate', icon: Wand2 },
  ]

  const placeholders: Record<AiTab, string> = {
    ask: 'Ask a question about this document...',
    improve: selectedText ? `Improving: "${selectedText.slice(0, 50)}${selectedText.length > 50 ? '…' : ''}"` : 'Paste text to improve, or select it in the editor...',
    summarize: 'Click Summarize to summarize the whole document',
    generate: 'Describe what to generate (e.g. bullet points about X, a table of Y)...',
  }

  return (
    <div
      ref={panelRef}
      className="absolute right-4 top-14 z-50 w-80 rounded-xl border border-zinc-700 bg-zinc-900 shadow-2xl"
      style={{ maxHeight: 'calc(100vh - 120px)' }}
    >
      {/* Header */}
      <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
        <Sparkles className="h-4 w-4 text-blue-400" />
        <span className="flex-1 text-sm font-medium text-zinc-200">AI Assistant</span>
        <button
          onClick={onClose}
          className="rounded p-0.5 text-zinc-600 hover:bg-zinc-800 hover:text-zinc-300 transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-zinc-800">
        {TABS.map((t) => {
          const Icon = t.icon
          return (
            <button
              key={t.id}
              onClick={() => { setTab(t.id); setResult(null); setError(null); setInput('') }}
              className={cn(
                'flex flex-1 items-center justify-center gap-1.5 py-2 text-xs font-medium transition-colors',
                tab === t.id
                  ? 'border-b-2 border-blue-500 text-blue-400'
                  : 'text-zinc-500 hover:text-zinc-300',
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {t.label}
            </button>
          )
        })}
      </div>

      {/* Content */}
      <div className="flex flex-col gap-3 p-4 overflow-y-auto" style={{ maxHeight: '380px' }}>
        {notConfigured ? (
          <div className="rounded-lg border border-zinc-800 bg-zinc-950/50 p-4 text-center">
            <Sparkles className="mx-auto mb-2 h-8 w-8 text-zinc-700" />
            <p className="text-sm font-medium text-zinc-400">AI not configured</p>
            <p className="mt-1 text-xs text-zinc-600">
              Configure an AI provider in settings to use this feature.
            </p>
          </div>
        ) : (
          <>
            {tab !== 'summarize' && (
              <textarea
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={placeholders[tab]}
                rows={3}
                className="w-full resize-none rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
              />
            )}

            {tab === 'improve' && selectedText && (
              <div className="rounded-lg border border-zinc-800 bg-zinc-950/50 px-3 py-2">
                <p className="text-[10px] font-medium uppercase tracking-wide text-zinc-600">Selected text</p>
                <p className="mt-1 text-xs text-zinc-400 line-clamp-3">{selectedText}</p>
              </div>
            )}

            {error && (
              <div className="rounded-lg border border-red-900/40 bg-red-950/20 px-3 py-2 text-xs text-red-400">
                {error}
              </div>
            )}

            {result && (
              <div className="rounded-lg border border-zinc-800 bg-zinc-950/50 px-3 py-3">
                <p className="whitespace-pre-wrap text-xs leading-relaxed text-zinc-300">{result}</p>
                {tab === 'generate' ? null : (
                  <button
                    onClick={() => {
                      editor.chain().focus().insertContent(result).run()
                      onClose()
                    }}
                    className="mt-2 text-xs text-blue-400 hover:text-blue-300"
                  >
                    Insert at cursor
                  </button>
                )}
              </div>
            )}

            <button
              onClick={handleAction}
              disabled={loading || (tab !== 'summarize' && !input.trim() && !selectedText)}
              className={cn(
                'flex w-full items-center justify-center gap-2 rounded-lg px-4 py-2 text-sm font-medium transition-colors',
                loading || (tab !== 'summarize' && !input.trim() && !selectedText)
                  ? 'bg-zinc-800 text-zinc-500 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-500',
              )}
            >
              {loading ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Processing...
                </>
              ) : (
                <>
                  <Send className="h-4 w-4" />
                  {tab === 'ask' ? 'Ask' : tab === 'improve' ? 'Improve' : tab === 'summarize' ? 'Summarize' : 'Generate'}
                </>
              )}
            </button>

            <p className="text-center text-[10px] text-zinc-700">⌘↵ to run · Esc to close</p>
          </>
        )}
      </div>
    </div>
  )
}
