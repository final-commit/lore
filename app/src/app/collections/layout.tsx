'use client'

import { Sidebar } from '@/components/sidebar/sidebar'
import { MobileSidebar } from '@/components/sidebar/mobile-sidebar'

export default function CollectionsLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex h-screen">
      <Sidebar className="hidden lg:flex" />
      <MobileSidebar />
      <main className="flex-1 overflow-y-auto">{children}</main>
    </div>
  )
}
