'use client'

import { useState, useEffect, Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import Link from 'next/link'
import { BookOpen } from 'lucide-react'
import { useAuth } from '@/contexts/auth-context'
import { api, setTokens } from '@/lib/api'
import type { OAuthProvider } from '@/lib/api'
import { cn } from '@/lib/utils'

// Separate component that uses useSearchParams (needs Suspense)
function OAuthRedirectHandler() {
  const searchParams = useSearchParams()
  useEffect(() => {
    const token = searchParams.get('token') ?? searchParams.get('access_token')
    const refresh = searchParams.get('refresh') ?? searchParams.get('refresh_token')
    if (token) {
      setTokens(token, refresh ?? '')
      window.location.href = '/'
    }
  }, [searchParams])
  return null
}

export default function LoginPage() {
  const { login } = useAuth()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [providers, setProviders] = useState<OAuthProvider[]>([])

  // Load OAuth providers
  useEffect(() => {
    api.auth.providers().then(setProviders).catch(() => {})
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!email.trim() || !password) return
    setLoading(true)
    setError(null)
    try {
      await login(email, password)
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  const enabledProviders = providers.filter((p) => p.enabled && p.configured)

  return (
    <div className="w-full max-w-sm">
      {/* Handle OAuth redirects */}
      <Suspense fallback={null}>
        <OAuthRedirectHandler />
      </Suspense>

      {/* Logo */}
      <div className="mb-8 flex items-center justify-center gap-2">
        <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-blue-600">
          <BookOpen className="h-5 w-5 text-white" />
        </div>
        <span className="text-xl font-semibold text-zinc-100">Lore</span>
      </div>

      <div className="rounded-xl border border-zinc-800 bg-zinc-900 p-8 shadow-2xl">
        <h1 className="mb-1 text-lg font-semibold text-zinc-100">Sign in</h1>
        <p className="mb-6 text-sm text-zinc-500">Welcome back to your knowledge base</p>

        {/* OAuth providers */}
        {enabledProviders.length > 0 && (
          <>
            <div className="mb-4 space-y-2">
              {enabledProviders.map((provider) => (
                <OAuthButton key={provider.provider} provider={provider.provider} />
              ))}
            </div>
            <div className="relative mb-4">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t border-zinc-800" />
              </div>
              <div className="relative flex justify-center">
                <span className="bg-zinc-900 px-3 text-xs text-zinc-600">or continue with email</span>
              </div>
            </div>
          </>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@company.com"
              required
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500/50"
            />
          </div>

          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="••••••••"
              required
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500/50"
            />
          </div>

          {error && (
            <div className="rounded-lg border border-red-900/50 bg-red-950/30 px-3 py-2 text-xs text-red-400">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={loading || !email.trim() || !password}
            className={cn(
              'w-full rounded-lg px-4 py-2.5 text-sm font-medium transition-colors',
              loading || !email.trim() || !password
                ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                : 'bg-blue-600 text-white hover:bg-blue-500',
            )}
          >
            {loading ? 'Signing in...' : 'Sign in'}
          </button>
        </form>

        <p className="mt-6 text-center text-xs text-zinc-600">
          No account?{' '}
          <Link href="/register" className="text-blue-400 hover:text-blue-300">
            Create one
          </Link>
        </p>
      </div>
    </div>
  )
}

function OAuthButton({ provider }: { provider: string }) {
  const label = provider.charAt(0).toUpperCase() + provider.slice(1)
  return (
    <a
      href={`/api/auth/oauth/${provider}`}
      className="flex w-full items-center justify-center gap-2.5 rounded-lg border border-zinc-700 bg-zinc-800 px-4 py-2.5 text-sm font-medium text-zinc-300 transition-colors hover:border-zinc-600 hover:bg-zinc-700"
    >
      <OAuthIcon provider={provider} />
      Continue with {label}
    </a>
  )
}

function OAuthIcon({ provider }: { provider: string }) {
  if (provider === 'google') {
    return (
      <svg className="h-4 w-4" viewBox="0 0 24 24">
        <path
          d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
          fill="#4285F4"
        />
        <path
          d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
          fill="#34A853"
        />
        <path
          d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
          fill="#FBBC05"
        />
        <path
          d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
          fill="#EA4335"
        />
      </svg>
    )
  }
  return <span className="h-4 w-4 text-center text-xs">{provider[0].toUpperCase()}</span>
}
