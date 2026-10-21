'use client'

import { useEffect, useState, useRef, useMemo } from 'react'
import { cn } from '@/lib/utils'

interface TocProps {
  content: string
  className?: string
}

interface TocItem {
  level: number
  text: string
  slug: string
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .trim()
}

export function TableOfContents({ content, className }: TocProps) {
  const [activeSlug, setActiveSlug] = useState<string>('')
  const observerRef = useRef<IntersectionObserver | null>(null)

  const headings = useMemo(() => {
    const items: TocItem[] = []
    const regex = /^(#{1,3})\s+(.+)$/gm
    let match
    while ((match = regex.exec(content)) !== null) {
      items.push({
        level: match[1].length,
        text: match[2].trim(),
        slug: slugify(match[2].trim()),
      })
    }
    return items
  }, [content])

  useEffect(() => {
    if (headings.length < 3) return

    observerRef.current = new IntersectionObserver(
      (entries) => {
        // Find the first visible heading
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
        if (visible.length > 0) {
          setActiveSlug(visible[0].target.id)
        }
      },
      { rootMargin: '-80px 0px -70% 0px', threshold: 0 },
    )

    const elements = document.querySelectorAll('h1[id], h2[id], h3[id]')
    elements.forEach((el) => observerRef.current?.observe(el))

    return () => observerRef.current?.disconnect()
  }, [headings])

  if (headings.length < 3) return null

  return (
    <nav className={cn('space-y-1', className)}>
      <p className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-zinc-600">
        On this page
      </p>
      {headings.map((h, i) => (
        <a
          key={`${h.slug}-${i}`}
          href={`#${h.slug}`}
          onClick={(e) => {
            e.preventDefault()
            const el = document.getElementById(h.slug)
            if (el) {
              el.scrollIntoView({ behavior: 'smooth', block: 'start' })
              setActiveSlug(h.slug)
            }
          }}
          className={cn(
            'block truncate py-1 text-xs transition-colors hover:text-zinc-300',
            h.level === 1 && 'font-bold',
            h.level === 2 && 'font-normal',
            h.level === 3 && 'pl-3 text-[11px]',
            activeSlug === h.slug ? 'text-zinc-200' : 'text-zinc-500',
          )}
        >
          {h.text}
        </a>
      ))}
    </nav>
  )
}
