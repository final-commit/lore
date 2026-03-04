'use client'

import { createContext, useContext, useEffect, useState, useCallback } from 'react'
import { useRouter, usePathname } from 'next/navigation'
import { api, setTokens, clearTokens, getAccessToken } from '@/lib/api'
import type { UserInfo } from '@/lib/api'

interface AuthContextValue {
  user: UserInfo | null
  loading: boolean
  login: (email: string, password: string) => Promise<void>
  register: (email: string, name: string, password: string) => Promise<void>
  logout: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

const PUBLIC_PATHS = ['/login', '/register', '/shared']

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<UserInfo | null>(null)
  const [loading, setLoading] = useState(true)
  const router = useRouter()
  const pathname = usePathname()

  const isPublicPath = PUBLIC_PATHS.some((p) => pathname.startsWith(p))

  const fetchMe = useCallback(async () => {
    const token = getAccessToken()
    if (!token) {
      setLoading(false)
      if (!isPublicPath) router.replace('/login')
      return
    }
    try {
      const me = await api.auth.me()
      setUser(me)
    } catch {
      clearTokens()
      setUser(null)
      if (!isPublicPath) router.replace('/login')
    } finally {
      setLoading(false)
    }
  }, [isPublicPath, router])

  useEffect(() => {
    fetchMe()
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // Redirect logged-in users away from auth pages
  useEffect(() => {
    if (!loading && user && (pathname === '/login' || pathname === '/register')) {
      router.replace('/docs')
    }
  }, [loading, user, pathname, router])

  const login = useCallback(async (email: string, password: string) => {
    const res = await api.auth.login(email, password)
    setTokens(res.access_token, res.refresh_token)
    setUser(res.user)
    router.replace('/docs')
  }, [router])

  const register = useCallback(async (email: string, name: string, password: string) => {
    const res = await api.auth.register(email, name, password)
    setTokens(res.access_token, res.refresh_token)
    setUser(res.user)
    router.replace('/docs')
  }, [router])

  const logout = useCallback(() => {
    clearTokens()
    setUser(null)
    router.replace('/login')
  }, [router])

  return (
    <AuthContext.Provider value={{ user, loading, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}
