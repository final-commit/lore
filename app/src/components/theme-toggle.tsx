'use client'

import { useEffect, useState } from 'react'
import { Sun, Moon } from 'lucide-react'
import { cn } from '@/lib/utils'

export function ThemeToggle() {
  const [dark, setDark] = useState(true)

  useEffect(() => {
    const stored = localStorage.getItem('forge-theme')
    if (stored === 'light') {
      setDark(false)
      document.documentElement.classList.remove('dark')
    }
  }, [])

  const toggle = () => {
    const next = !dark
    setDark(next)
    if (next) {
      document.documentElement.classList.add('dark')
      localStorage.setItem('forge-theme', 'dark')
    } else {
      document.documentElement.classList.remove('dark')
      localStorage.setItem('forge-theme', 'light')
    }
  }

  return (
    <button
      onClick={toggle}
      className="rounded-md p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300 dark:hover:bg-zinc-800"
      title={dark ? 'Switch to light mode' : 'Switch to dark mode'}
    >
      {dark ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
    </button>
  )
}
