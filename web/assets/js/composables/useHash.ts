import { useRouteHash } from '@vueuse/router'
import type { RouteLocationNormalizedLoaded } from 'vue-router'
import { computed } from 'vue'
import { activeRouter } from '@/router'

/** The current route as @vueuse/router reads it, for use outside components too. */
const liveRoute = () => {
  const current = () => activeRouter().currentRoute.value

  return {
    get hash() {
      return current().hash
    },
    get params() {
      return current().params
    },
    get query() {
      return current().query
    },
  } as RouteLocationNormalizedLoaded
}

/**
 * The URL's hash, without the `#`, for what a page shows within itself (its
 * tab: `/settings#admin-users`), so a reload or a shared link opens it again.
 * Setting it replaces the URL; the screen doesn't refetch for it.
 */
export const useHash = () => {
  const hash = useRouteHash('', { mode: 'replace', route: liveRoute(), router: activeRouter() })

  return computed({
    get: () => decodeURIComponent((hash.value ?? '').replace(/^#/, '')),
    set: value => (hash.value = value ? `#${value}` : null),
  })
}

/** A page's tab, kept in the hash: one of `tabs`, or `fallback` when the hash names none. */
export const useHashTab = <T extends string>(tabs: readonly T[], fallback: T) => {
  const hash = useHash()

  return computed<T>({
    get: () => (tabs.includes(hash.value as T) ? (hash.value as T) : fallback),
    set: tab => (hash.value = tab),
  })
}
