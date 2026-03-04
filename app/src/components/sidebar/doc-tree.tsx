'use client'

import { useState } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { ChevronRight, FileText, FolderOpen, Folder } from 'lucide-react'
import { cn } from '@/lib/utils'

export interface TreeNode {
  name: string
  path: string
  title: string
  isDirectory: boolean
  order: number
  children?: TreeNode[]
  hidden?: boolean
}

interface DocTreeProps {
  nodes: TreeNode[]
  level?: number
}

export function DocTree({ nodes, level = 0 }: DocTreeProps) {
  return (
    <ul className={cn('space-y-0.5', level > 0 && 'ml-3 border-l border-zinc-200 pl-2 dark:border-zinc-800')}>
      {nodes
        .filter((n) => !n.hidden)
        .map((node) => (
          <DocTreeItem key={node.path} node={node} level={level} />
        ))}
    </ul>
  )
}

function DocTreeItem({ node, level }: { node: TreeNode; level: number }) {
  const pathname = usePathname()
  const [expanded, setExpanded] = useState(level === 0)
  const isActive = pathname === `/docs/${node.path.replace(/\.mdx?$/, '')}`

  if (node.isDirectory) {
    return (
      <li>
        <button
          onClick={() => setExpanded(!expanded)}
          className={cn(
            'flex w-full items-center gap-1.5 rounded-md px-2 py-1.5 text-sm font-medium',
            'text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-800/50 dark:hover:text-zinc-200',
            'transition-colors duration-150',
          )}
        >
          <ChevronRight
            className={cn('h-3.5 w-3.5 shrink-0 transition-transform', expanded && 'rotate-90')}
          />
          {expanded ? (
            <FolderOpen className="h-4 w-4 shrink-0 text-zinc-500" />
          ) : (
            <Folder className="h-4 w-4 shrink-0 text-zinc-500" />
          )}
          <span className="truncate">{node.title}</span>
        </button>
        {expanded && node.children && <DocTree nodes={node.children} level={level + 1} />}
      </li>
    )
  }

  const href = `/docs/${node.path.replace(/\.mdx?$/, '')}`

  return (
    <li>
      <Link
        href={href}
        className={cn(
          'flex items-center gap-1.5 rounded-md px-2 py-1.5 text-sm',
          'transition-colors duration-150',
          isActive
            ? 'bg-zinc-200 font-medium text-zinc-900 dark:bg-zinc-800 dark:text-zinc-100'
            : 'text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:text-zinc-400 dark:hover:bg-zinc-800/50 dark:hover:text-zinc-200',
        )}
      >
        <FileText className="h-4 w-4 shrink-0 text-zinc-500" />
        <span className="truncate">{node.title}</span>
      </Link>
    </li>
  )
}
