import { useEventListener } from '@vueuse/core'
import { computed, ref, watch } from 'vue'

const read = () => decodeURIComponent(location.hash.replace(/^#/, ''))

/**
 * The URL's hash, without the `#`, for what a page shows within itself (its
 * tab: `/settings#admin-users`), so a reload or a shared link opens it again.
 * Like @vueuse/router's `useRouteHash`, setting it replaces the URL: no history
 * entry, and no navigation for the router to answer.
 */
export const useHash = () => {
  const hash = ref(read())

  // Typed in the address bar, or arrived with a navigation (the router's
  // own pushes announce themselves as popstate).
  useEventListener(window, 'hashchange', () => (hash.value = read()))
  useEventListener(window, 'popstate', () => (hash.value = read()))

  watch(hash, value => {
    if (value === read()) {
      return
    }

    const fragment = value ? `#${encodeURIComponent(value)}` : ''
    history.replaceState(history.state, '', `${location.pathname}${location.search}${fragment}`)
  })

  return hash
}

/** A page's tab, kept in the hash: one of `tabs`, or `fallback` when the hash names none. */
export const useHashTab = <T extends string>(tabs: readonly T[], fallback: T) => {
  const hash = useHash()

  return computed<T>({
    get: () => (tabs.includes(hash.value as T) ? (hash.value as T) : fallback),
    set: tab => (hash.value = tab),
  })
}

/**
 * Moves a tab named the old way, in the query (`?tab=users`) or at the end of
 * the path (`/albums/al-1/information`), to the hash (`#admin-users`).
 */
export const moveTabToHash = (tab: string, hash = tab) => {
  const url = new URL(location.href)
  url.searchParams.delete('tab')
  url.pathname = url.pathname.replace(new RegExp(`/${tab}/?$`), '')
  url.hash = hash
  history.replaceState(history.state, '', url)
}
