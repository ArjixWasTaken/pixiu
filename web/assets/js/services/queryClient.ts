import { QueryClient } from '@tanstack/vue-query'

/**
 * What the player fetched from the server, and how fresh it is: TanStack
 * Query keeps it, fetches each thing once however many ask at a time, and
 * fetches again once it's stale or invalidated (a song edited, a job done).
 *
 * Results that are entities (songs, albums, artists) go through their store's
 * vault first, so every list shows the same object; hence `shallow` (the
 * entities stay writable) and no structural sharing (it would copy them).
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60_000,
      refetchOnWindowFocus: false,
      structuralSharing: false,
      shallow: true,
    },
  },
})
