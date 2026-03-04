'use client'

import { Node, mergeAttributes, nodePasteRule } from '@tiptap/core'
import { NodeViewWrapper, ReactNodeViewRenderer } from '@tiptap/react'
import type { NodeViewProps } from '@tiptap/react'
import { useState, useEffect } from 'react'
import { ExternalLink, AlertCircle, Globe } from 'lucide-react'
import { api } from '@/lib/api'
import type { UnfurlResult } from '@/lib/api'
import { cn } from '@/lib/utils'

// ── NodeView component ────────────────────────────────────────────────────────

function EmbedView({ node, updateAttributes, selected }: NodeViewProps) {
  const { url, title, description, image, favicon, embedHtml, loaded } = node.attrs as {
    url: string
    title: string | null
    description: string | null
    image: string | null
    favicon: string | null
    embedHtml: string | null
    loaded: boolean
  }

  const [fetching, setFetching] = useState(!loaded && !!url)
  const [fetchError, setFetchError] = useState(false)

  useEffect(() => {
    if (loaded || !url) return
    setFetching(true)
    setFetchError(false)
    api.unfurl
      .url(url)
      .then((data: UnfurlResult) => {
        updateAttributes({
          title: data.title ?? null,
          description: data.description ?? null,
          image: data.image ?? null,
          favicon: data.favicon ?? null,
          embedHtml: data.embed_html ?? null,
          loaded: true,
        })
      })
      .catch(() => {
        setFetchError(true)
        updateAttributes({ loaded: true })
      })
      .finally(() => setFetching(false))
  }, [url, loaded, updateAttributes])

  return (
    <NodeViewWrapper
      className={cn(
        'my-3 block rounded-lg border transition-colors',
        selected ? 'border-blue-500/50' : 'border-zinc-800',
      )}
      contentEditable={false}
    >
      {fetching && <EmbedSkeleton url={url} />}
      {!fetching && (fetchError || (!title && !embedHtml)) && (
        <EmbedFallback url={url} error={fetchError} />
      )}
      {!fetching && !fetchError && embedHtml && (
        <EmbedIframe html={embedHtml} />
      )}
      {!fetching && !fetchError && title && !embedHtml && (
        <EmbedCard url={url} title={title} description={description} image={image} favicon={favicon} />
      )}
    </NodeViewWrapper>
  )
}

function EmbedSkeleton({ url }: { url: string }) {
  return (
    <div className="flex items-center gap-3 p-4">
      <div className="h-10 w-10 animate-pulse rounded bg-zinc-800" />
      <div className="flex-1 space-y-2">
        <div className="h-3 w-2/3 animate-pulse rounded bg-zinc-800" />
        <div className="h-2.5 w-1/2 animate-pulse rounded bg-zinc-800" />
        <div className="h-2 w-1/3 animate-pulse rounded bg-zinc-800" />
      </div>
      <span className="text-xs text-zinc-600 truncate max-w-[120px]">{url}</span>
    </div>
  )
}

function EmbedFallback({ url, error }: { url: string; error: boolean }) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="flex items-center gap-2 p-4 text-sm text-blue-400 hover:text-blue-300 transition-colors"
    >
      {error ? (
        <AlertCircle className="h-4 w-4 shrink-0 text-zinc-600" />
      ) : (
        <Globe className="h-4 w-4 shrink-0 text-zinc-600" />
      )}
      <span className="truncate">{url}</span>
      <ExternalLink className="h-3.5 w-3.5 shrink-0 text-zinc-600" />
    </a>
  )
}

function EmbedIframe({ html }: { html: string }) {
  return (
    <div
      className="embed-iframe-wrapper overflow-hidden rounded-lg"
      dangerouslySetInnerHTML={{ __html: html }}
      style={{ position: 'relative', paddingBottom: '56.25%', height: 0 }}
    />
  )
}

function EmbedCard({
  url, title, description, image, favicon,
}: {
  url: string
  title: string
  description: string | null
  image: string | null
  favicon: string | null
}) {
  const domain = (() => {
    try { return new URL(url).hostname } catch { return url }
  })()

  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="flex items-stretch gap-0 overflow-hidden rounded-lg hover:bg-zinc-900/50 transition-colors"
    >
      {image && (
        <div className="w-28 shrink-0 overflow-hidden">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src={image} alt="" className="h-full w-full object-cover" />
        </div>
      )}
      <div className="flex flex-1 flex-col justify-center gap-1 p-4 min-w-0">
        <p className="text-sm font-medium text-zinc-200 line-clamp-2">{title}</p>
        {description && (
          <p className="text-xs text-zinc-500 line-clamp-2">{description}</p>
        )}
        <div className="mt-1 flex items-center gap-1.5">
          {favicon ? (
            // eslint-disable-next-line @next/next/no-img-element
            <img src={favicon} alt="" className="h-3 w-3 rounded-sm" />
          ) : (
            <Globe className="h-3 w-3 text-zinc-600" />
          )}
          <span className="text-[11px] text-zinc-500">{domain}</span>
        </div>
      </div>
    </a>
  )
}

// ── Tiptap Extension ──────────────────────────────────────────────────────────

export const EmbedExtension = Node.create({
  name: 'embed',
  group: 'block',
  atom: true,
  draggable: true,
  selectable: true,

  addAttributes() {
    return {
      url: { default: '' },
      title: { default: null },
      description: { default: null },
      image: { default: null },
      favicon: { default: null },
      embedHtml: { default: null },
      loaded: { default: false },
    }
  },

  parseHTML() {
    return [
      {
        tag: 'div[data-type="embed"]',
        getAttrs: (el) => {
          const div = el as HTMLElement
          return {
            url: div.getAttribute('data-url') ?? '',
            title: div.getAttribute('data-title'),
            description: div.getAttribute('data-description'),
            image: div.getAttribute('data-image'),
            favicon: div.getAttribute('data-favicon'),
            embedHtml: div.getAttribute('data-embed-html'),
            loaded: div.getAttribute('data-loaded') === 'true',
          }
        },
      },
    ]
  },

  renderHTML({ HTMLAttributes }) {
    return [
      'div',
      mergeAttributes(
        {
          'data-type': 'embed',
          'data-url': HTMLAttributes.url,
          'data-title': HTMLAttributes.title,
          'data-description': HTMLAttributes.description,
          'data-image': HTMLAttributes.image,
          'data-favicon': HTMLAttributes.favicon,
          'data-embed-html': HTMLAttributes.embedHtml,
          'data-loaded': HTMLAttributes.loaded ? 'true' : 'false',
        },
        { class: 'embed-block' },
      ),
    ]
  },

  addNodeView() {
    return ReactNodeViewRenderer(EmbedView)
  },

  addPasteRules() {
    return [
      nodePasteRule({
        find: /^https?:\/\/[^\s]+$/gm,
        type: this.type,
        getAttributes: (match) => ({ url: match[0].trim(), loaded: false }),
      }),
    ]
  },

  addCommands() {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return {
      insertEmbed:
        (url: string) =>
        ({ commands }: { commands: { insertContent: (c: unknown) => boolean } }) => {
          return commands.insertContent({
            type: this.name,
            attrs: { url, loaded: false },
          })
        },
    } as unknown as ReturnType<NonNullable<typeof this.parent>>
  },
})

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    embed: {
      insertEmbed: (url: string) => ReturnType
    }
  }
}
