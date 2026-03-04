'use client'

import { useEffect, useState, useCallback } from 'react'
import { MessageSquare, Check, Reply, X, Send, Bot, SmilePlus } from 'lucide-react'
import { api } from '@/lib/api'
import { cn } from '@/lib/utils'
import type { Comment } from '@/lib/api'

interface CommentPanelProps {
  filePath: string
  open: boolean
  onClose: () => void
}

export function CommentPanel({ filePath, open, onClose }: CommentPanelProps) {
  const [comments, setComments] = useState<Comment[]>([])
  const [loading, setLoading] = useState(true)
  const [newComment, setNewComment] = useState('')
  const [replyingTo, setReplyingTo] = useState<string | null>(null)
  const [replyText, setReplyText] = useState('')

  const threads = comments.filter((c) => !c.parent_id)
  const getReplies = (id: string) => comments.filter((c) => c.parent_id === id)

  const loadComments = useCallback(async () => {
    setLoading(true)
    try {
      const data = await api.comments.list(filePath)
      setComments(data)
    } catch {
      setComments([])
    } finally {
      setLoading(false)
    }
  }, [filePath])

  useEffect(() => {
    if (open) loadComments()
  }, [open, loadComments])

  const handleCreateComment = async () => {
    if (!newComment.trim()) return
    try {
      await api.comments.create(filePath, newComment)
      setNewComment('')
      loadComments()
    } catch { /* ignore */ }
  }

  const handleReply = async (parentId: string) => {
    if (!replyText.trim()) return
    try {
      await api.comments.create(filePath, replyText, { parent_id: parentId })
      setReplyingTo(null)
      setReplyText('')
      loadComments()
    } catch { /* ignore */ }
  }

  const handleResolve = async (commentId: string, currentlyResolved: boolean) => {
    try {
      if (currentlyResolved) {
        await api.comments.unresolve(commentId)
      } else {
        await api.comments.resolve(commentId)
      }
      loadComments()
    } catch { /* ignore */ }
  }

  const handleReact = async (commentId: string, emoji: string) => {
    try {
      await api.comments.react(commentId, emoji)
      loadComments()
    } catch { /* ignore */ }
  }

  if (!open) return null

  return (
    <aside className="flex h-full w-80 flex-col border-l border-zinc-800 bg-zinc-950">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-zinc-800 px-4 py-3">
        <div className="flex items-center gap-2">
          <MessageSquare className="h-4 w-4 text-zinc-400" />
          <span className="text-sm font-medium text-zinc-200">Comments</span>
          {threads.length > 0 && (
            <span className="rounded-full bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-400">
              {threads.length}
            </span>
          )}
        </div>
        <button onClick={onClose} className="text-zinc-500 hover:text-zinc-300 transition-colors">
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* Comment list */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
          </div>
        ) : threads.length === 0 ? (
          <div className="flex flex-col items-center justify-center px-4 py-12 text-center">
            <MessageSquare className="h-8 w-8 text-zinc-700 mb-2" />
            <p className="text-sm text-zinc-600">No comments yet</p>
            <p className="mt-1 text-xs text-zinc-700">Start a conversation below</p>
          </div>
        ) : (
          <div className="divide-y divide-zinc-800/50">
            {threads.map((comment) => (
              <CommentThread
                key={comment.id}
                comment={comment}
                replies={getReplies(comment.id)}
                replyingTo={replyingTo}
                replyText={replyText}
                onStartReply={() => { setReplyingTo(comment.id); setReplyText('') }}
                onCancelReply={() => setReplyingTo(null)}
                onReplyTextChange={setReplyText}
                onReply={() => handleReply(comment.id)}
                onResolve={() => handleResolve(comment.id, !!comment.resolved_at)}
                onReact={(emoji) => handleReact(comment.id, emoji)}
              />
            ))}
          </div>
        )}
      </div>

      {/* New comment input */}
      <div className="border-t border-zinc-800 p-3">
        <textarea
          value={newComment}
          onChange={(e) => setNewComment(e.target.value)}
          placeholder="Add a comment..."
          className="w-full resize-none rounded-lg border border-zinc-800 bg-zinc-900 px-3 py-2 text-sm text-zinc-300 placeholder:text-zinc-600 focus:border-zinc-600 focus:outline-none"
          rows={3}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') handleCreateComment()
          }}
        />
        <div className="mt-2 flex items-center justify-between">
          <span className="text-[10px] text-zinc-700">⌘↵ to submit</span>
          <button
            onClick={handleCreateComment}
            disabled={!newComment.trim()}
            className={cn(
              'flex items-center gap-1 rounded-lg px-3 py-1 text-xs font-medium transition-colors',
              newComment.trim()
                ? 'bg-blue-600 text-white hover:bg-blue-500'
                : 'bg-zinc-800 text-zinc-600 cursor-not-allowed',
            )}
          >
            <Send className="h-3 w-3" />
            Comment
          </button>
        </div>
      </div>
    </aside>
  )
}

// ── Thread ────────────────────────────────────────────────────────────────────

const REACTION_EMOJIS = ['👍', '❤️', '😂']

function CommentThread({
  comment, replies, replyingTo, replyText,
  onStartReply, onCancelReply, onReplyTextChange, onReply, onResolve, onReact,
}: {
  comment: Comment
  replies: Comment[]
  replyingTo: string | null
  replyText: string
  onStartReply: () => void
  onCancelReply: () => void
  onReplyTextChange: (text: string) => void
  onReply: () => void
  onResolve: () => void
  onReact: (emoji: string) => void
}) {
  const isReplying = replyingTo === comment.id
  const isResolved = !!comment.resolved_at
  const [showReactions, setShowReactions] = useState(false)

  return (
    <div className={cn('px-4 py-3', isResolved && 'opacity-60')}>
      <CommentBubble comment={comment} />

      {comment.anchor_text && (
        <div className="mt-1">
          <span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-500 italic">
            &ldquo;{comment.anchor_text.slice(0, 50)}{comment.anchor_text.length > 50 ? '…' : ''}&rdquo;
          </span>
        </div>
      )}

      {comment.reactions && Object.keys(comment.reactions).length > 0 && (
        <div className="mt-1.5 flex flex-wrap gap-1">
          {Object.entries(comment.reactions).map(([emoji, count]) => (
            <button
              key={emoji}
              onClick={() => onReact(emoji)}
              className="flex items-center gap-0.5 rounded-full border border-zinc-800 bg-zinc-900 px-1.5 py-0.5 text-xs hover:border-zinc-700 transition-colors"
            >
              <span>{emoji}</span>
              <span className="text-zinc-500">{count}</span>
            </button>
          ))}
        </div>
      )}

      {replies.length > 0 && (
        <div className="mt-2 ml-3 space-y-2 border-l border-zinc-800 pl-3">
          {replies.map((reply) => (
            <CommentBubble key={reply.id} comment={reply} />
          ))}
        </div>
      )}

      {isReplying && (
        <div className="mt-2 ml-3 border-l border-zinc-800 pl-3">
          <textarea
            value={replyText}
            onChange={(e) => onReplyTextChange(e.target.value)}
            placeholder="Write a reply..."
            className="w-full resize-none rounded border border-zinc-800 bg-zinc-900 px-2 py-1.5 text-xs text-zinc-300 placeholder:text-zinc-600 focus:border-zinc-600 focus:outline-none"
            rows={2}
            autoFocus
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') onReply()
              if (e.key === 'Escape') onCancelReply()
            }}
          />
          <div className="mt-1 flex gap-1">
            <button
              onClick={onReply}
              disabled={!replyText.trim()}
              className="rounded px-2 py-0.5 text-[10px] font-medium bg-blue-600 text-white hover:bg-blue-500 disabled:bg-zinc-800 disabled:text-zinc-600"
            >
              Reply
            </button>
            <button onClick={onCancelReply} className="rounded px-2 py-0.5 text-[10px] text-zinc-500 hover:text-zinc-300">
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="mt-2 flex items-center gap-2">
        <button
          onClick={onStartReply}
          className="flex items-center gap-1 text-[10px] text-zinc-600 hover:text-zinc-400 transition-colors"
        >
          <Reply className="h-3 w-3" />
          Reply
        </button>
        <button
          onClick={onResolve}
          className={cn(
            'flex items-center gap-1 text-[10px] transition-colors',
            isResolved ? 'text-green-500 hover:text-green-400' : 'text-zinc-600 hover:text-zinc-400',
          )}
        >
          <Check className="h-3 w-3" />
          {isResolved ? 'Resolved' : 'Resolve'}
        </button>
        <div className="relative ml-auto">
          <button
            onClick={() => setShowReactions(!showReactions)}
            className="text-[10px] text-zinc-600 hover:text-zinc-400 transition-colors"
          >
            <SmilePlus className="h-3 w-3" />
          </button>
          {showReactions && (
            <div className="absolute bottom-full right-0 mb-1 flex gap-1 rounded-lg border border-zinc-700 bg-zinc-900 p-1.5 shadow-lg z-10">
              {REACTION_EMOJIS.map((emoji) => (
                <button
                  key={emoji}
                  onClick={() => { onReact(emoji); setShowReactions(false) }}
                  className="rounded px-1 py-0.5 text-base hover:bg-zinc-800 transition-colors"
                >
                  {emoji}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Bubble ────────────────────────────────────────────────────────────────────

function CommentBubble({ comment }: { comment: Comment }) {
  const displayName = comment.is_agent
    ? (comment.agent_name ?? 'Agent')
    : (comment.author_name ?? comment.author_id)

  return (
    <div>
      <div className="flex items-center gap-1.5">
        {comment.is_agent && <Bot className="h-3 w-3 text-purple-400" />}
        <span className={cn('text-xs font-medium', comment.is_agent ? 'text-purple-300' : 'text-zinc-300')}>
          {displayName}
        </span>
        <span className="text-[10px] text-zinc-600">{getTimeAgo(comment.created_at)}</span>
      </div>
      <p className="mt-0.5 text-xs leading-relaxed text-zinc-400 whitespace-pre-wrap">{comment.body}</p>
    </div>
  )
}

function getTimeAgo(dateStr: string): string {
  const diffMs = Date.now() - new Date(dateStr).getTime()
  const diffMin = Math.floor(diffMs / 60000)
  if (diffMin < 1) return 'just now'
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHr = Math.floor(diffMin / 60)
  if (diffHr < 24) return `${diffHr}h ago`
  return `${Math.floor(diffHr / 24)}d ago`
}
