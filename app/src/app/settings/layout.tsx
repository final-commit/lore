'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { User, Palette, Users, Settings, Key, Keyboard, ArrowLeft, GroupIcon, Plug, Smile } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useAuth } from '@/contexts/auth-context'

const NAV_ITEMS = [
  { label: 'Profile', href: '/settings/profile', icon: User },
  { label: 'Preferences', href: '/settings/preferences', icon: Palette },
  { label: 'Team', href: '/settings/team', icon: Settings, adminOnly: true },
  { label: 'Members', href: '/settings/members', icon: Users, adminOnly: true },
  { label: 'Groups', href: '/settings/groups', icon: GroupIcon, adminOnly: true },
  { label: 'Integrations', href: '/settings/integrations', icon: Plug, adminOnly: true },
  { label: 'Emoji', href: '/settings/emojis', icon: Smile, adminOnly: true },
  { label: 'API Tokens', href: '/settings/api-tokens', icon: Key },
  { label: 'Shortcuts', href: '/settings/shortcuts', icon: Keyboard },
]

export default function SettingsLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  const { user } = useAuth()
  const isAdmin = user?.role === 'admin'

  const visibleItems = NAV_ITEMS.filter((item) => !item.adminOnly || isAdmin)

  return (
    <div className="flex min-h-screen bg-zinc-950">
      {/* Sidebar */}
      <aside className="w-56 shrink-0 border-r border-zinc-800 px-3 py-6">
        <Link
          href="/docs"
          className="mb-6 flex items-center gap-1.5 px-2 text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          Back to docs
        </Link>
        <p className="mb-3 px-2 text-[10px] font-semibold uppercase tracking-wider text-zinc-600">Settings</p>
        <nav className="space-y-0.5">
          {visibleItems.map((item) => {
            const Icon = item.icon
            const isActive = pathname === item.href
            return (
              <Link
                key={item.href}
                href={item.href}
                className={cn(
                  'flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors',
                  isActive
                    ? 'bg-zinc-800 text-zinc-100 font-medium'
                    : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200',
                )}
              >
                <Icon className="h-4 w-4 shrink-0" />
                {item.label}
              </Link>
            )
          })}
        </nav>
      </aside>

      {/* Content */}
      <main className="flex-1 overflow-y-auto">
        {children}
      </main>
    </div>
  )
}
