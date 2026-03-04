'use client'

import { useState, useEffect } from 'react'
import { api } from '@/lib/api'
import type { OAuthProvider } from '@/lib/api'
import { cn } from '@/lib/utils'
import { useToast } from '@/contexts/toast-context'
import { CheckCircle2, AlertCircle, Loader2 } from 'lucide-react'

export default function IntegrationsPage() {
  const { toast } = useToast()
  const [providers, setProviders] = useState<OAuthProvider[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api.auth
      .providers()
      .then(setProviders)
      .catch(() => {})
      .finally(() => setLoading(false))
  }, [])

  const refreshProviders = () => {
    api.auth.providers().then(setProviders).catch(() => {})
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-zinc-700 border-t-zinc-400" />
      </div>
    )
  }

  // Ensure Google is always shown (may not be in providers list if never configured)
  const allProviders: OAuthProvider[] = [
    providers.find((p) => p.provider === 'google') ?? {
      provider: 'google',
      enabled: false,
      configured: false,
    },
    ...providers.filter((p) => p.provider !== 'google'),
  ]

  return (
    <div className="max-w-2xl px-8 py-10">
      <h1 className="mb-1 text-xl font-semibold text-zinc-100">Integrations</h1>
      <p className="mb-8 text-sm text-zinc-500">Connect external services to extend Forge</p>

      <div className="space-y-4">
        {allProviders.map((provider) => (
          <ProviderTile
            key={provider.provider}
            provider={provider}
            onSaved={refreshProviders}
            toast={toast}
          />
        ))}

        {/* Future integrations placeholder */}
        <div className="rounded-xl border border-dashed border-zinc-800 p-6 text-center">
          <p className="text-sm text-zinc-600">More integrations coming soon</p>
        </div>
      </div>
    </div>
  )
}

function ProviderTile({
  provider,
  onSaved,
  toast,
}: {
  provider: OAuthProvider
  onSaved: () => void
  toast: (msg: string, type?: 'success' | 'error' | 'info' | 'warning') => void
}) {
  const [expanded, setExpanded] = useState(false)
  const [clientId, setClientId] = useState(provider.client_id ?? '')
  const [clientSecret, setClientSecret] = useState('')
  const [enabled, setEnabled] = useState(provider.enabled)
  const [saving, setSaving] = useState(false)

  const PROVIDER_META: Record<string, { name: string; description: string; icon: React.ReactNode }> = {
    google: {
      name: 'Google OAuth',
      description: 'Allow team members to sign in with their Google accounts.',
      icon: (
        <svg className="h-6 w-6" viewBox="0 0 24 24">
          <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4" />
          <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853" />
          <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" fill="#FBBC05" />
          <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335" />
        </svg>
      ),
    },
  }

  const meta = PROVIDER_META[provider.provider] ?? {
    name: provider.provider.charAt(0).toUpperCase() + provider.provider.slice(1),
    description: `OAuth integration for ${provider.provider}`,
    icon: <span className="text-2xl">{provider.provider[0].toUpperCase()}</span>,
  }

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault()
    setSaving(true)
    try {
      await api.auth.configureProvider(provider.provider, {
        client_id: clientId,
        client_secret: clientSecret,
        enabled,
      })
      toast(`${meta.name} integration saved`, 'success')
      onSaved()
      setExpanded(false)
    } catch (err: unknown) {
      toast(err instanceof Error ? err.message : 'Failed to save', 'error')
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900/50">
      {/* Tile header */}
      <div className="flex items-center gap-4 p-5">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-zinc-800 bg-zinc-900">
          {meta.icon}
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-zinc-200">{meta.name}</p>
          <p className="text-xs text-zinc-500">{meta.description}</p>
        </div>
        <div className="flex items-center gap-3">
          {/* Status badge */}
          {provider.configured ? (
            <span className="flex items-center gap-1 rounded-full bg-green-900/30 px-2.5 py-1 text-[11px] font-medium text-green-400">
              <CheckCircle2 className="h-3 w-3" />
              Configured
            </span>
          ) : (
            <span className="flex items-center gap-1 rounded-full bg-zinc-800 px-2.5 py-1 text-[11px] font-medium text-zinc-500">
              <AlertCircle className="h-3 w-3" />
              Not set up
            </span>
          )}
          <button
            onClick={() => setExpanded((v) => !v)}
            className="rounded-lg border border-zinc-700 px-3 py-1.5 text-xs text-zinc-400 transition-colors hover:border-zinc-600 hover:text-zinc-200"
          >
            {expanded ? 'Cancel' : 'Configure'}
          </button>
        </div>
      </div>

      {/* Expanded config form */}
      {expanded && (
        <form onSubmit={handleSave} className="border-t border-zinc-800 p-5 space-y-4">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">Client ID</label>
            <input
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              placeholder="Paste your Client ID"
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs font-medium text-zinc-400">
              Client Secret {provider.configured && <span className="text-zinc-600">(leave blank to keep existing)</span>}
            </label>
            <input
              type="password"
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
              placeholder={provider.configured ? '••••••••' : 'Paste your Client Secret'}
              className="w-full rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-600 focus:border-blue-500 focus:outline-none"
            />
          </div>

          {/* Enable toggle */}
          <div className="flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-950/30 px-4 py-3">
            <div>
              <p className="text-sm text-zinc-300">Enable {meta.name}</p>
              <p className="text-xs text-zinc-600">Allow users to sign in via this provider</p>
            </div>
            <button
              type="button"
              onClick={() => setEnabled((v) => !v)}
              className={cn(
                'relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors',
                enabled ? 'bg-blue-600' : 'bg-zinc-700',
              )}
            >
              <span
                className={cn(
                  'inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow transition-transform',
                  enabled ? 'translate-x-4' : 'translate-x-0.5',
                )}
              />
            </button>
          </div>

          <div className="flex items-center gap-3">
            <button
              type="submit"
              disabled={saving || !clientId.trim()}
              className={cn(
                'rounded-lg px-4 py-2 text-sm font-medium transition-colors',
                saving || !clientId.trim()
                  ? 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                  : 'bg-blue-600 text-white hover:bg-blue-500',
              )}
            >
              {saving ? (
                <span className="flex items-center gap-2">
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  Saving...
                </span>
              ) : (
                'Save integration'
              )}
            </button>
            <p className="text-xs text-zinc-600">
              Redirect URI: <code className="font-mono">/api/auth/oauth/{provider.provider}/callback</code>
            </p>
          </div>
        </form>
      )}
    </div>
  )
}
