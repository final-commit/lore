import { createTRPCReact, type CreateTRPCReact } from '@trpc/react-query'
import type { AnyTRPCRouter } from '@trpc/server'

// The AppRouter type will be properly imported once we set up
// a shared type export. For now, we create an untyped client
// that still works at runtime — type safety comes when we wire
// the proper AppRouter type from @forge/server.
export const trpc: CreateTRPCReact<AnyTRPCRouter, unknown> = createTRPCReact<AnyTRPCRouter>()
