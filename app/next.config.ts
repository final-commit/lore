import type { NextConfig } from 'next'

const backendUrl = process.env.LORE_BACKEND_URL || 'http://localhost:3334'

const nextConfig: NextConfig = {
  output: 'standalone',
  transpilePackages: ['@forge/shared'],
  // Proxy API + WebSocket requests to the Rust backend
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${backendUrl}/api/:path*`,
      },
      {
        source: '/health',
        destination: `${backendUrl}/health`,
      },
      {
        source: '/ws/:path*',
        destination: `${backendUrl}/ws/:path*`,
      },
    ]
  },
}

export default nextConfig
