import { ref, shallowRef } from 'vue'
import type { RouteLocationNormalizedLoaded, Router as VueRouter, RouterHistory } from 'vue-router'
import { createRouter, createWebHistory } from 'vue-router'
import type { RouteName } from '@/config/routes'
import { routes } from '@/config/routes'
import { forceReloadWindow } from '@/utils/helpers'
import { basePath, toClientPath } from '@/utils/clientUrl'

/** A route as the screens know it (see `useRouter`). */
export interface Route {
  name?: string
  path: string
  screen: ScreenName
  /** The path's parameters, and the query's. */
  params: Record<string, string>
}

type RouteChangedHandler = (route: Route, previous: Route | undefined) => unknown

const first = (value: unknown) => (Array.isArray(value) ? value[0] : value)

const stringify = (record: Record<string, unknown>) =>
  Object.fromEntries(
    Object.entries(record)
      .map(([key, value]) => [key, first(value)])
      .filter(([, value]) => value !== null && value !== undefined)
      .map(([key, value]) => [key, String(value)]),
  ) as Record<string, string>

export const toRoute = (location: RouteLocationNormalizedLoaded): Route => ({
  name: location.name as string | undefined,
  path: location.path,
  screen: location.meta.screen ?? '404',
  params: { ...stringify(location.query), ...stringify(location.params) },
})

/** The router in use: the app's, or a spec's. */
const active = shallowRef<VueRouter>()

/** Set when what a screen shows doesn't exist, or a guard says no: the 404 screen shows, the address stays. */
export const notFound = ref(false)

const handlers = new Set<RouteChangedHandler>()
let guarding = false

const denied = (location: RouteLocationNormalizedLoaded) => guarding && location.meta.guard?.() === false

/** Old links had the path in the hash (`/#/albums/al-1`): they become plain paths. */
const rewriteHashUrl = () => {
  const legacy = location.hash.match(/^#!?\//)

  if (legacy) {
    history.replaceState(history.state, '', `${basePath()}${location.hash.substring(legacy[0].length)}`)
  }
}

export const createAppRouter = (history?: RouterHistory) => {
  if (!history) {
    rewriteHashUrl()
    history = createWebHistory(basePath())
  }

  const router = createRouter({ history, routes: [...routes] })

  router.afterEach((to, from, failure) => {
    if (failure) {
      return
    }

    notFound.value = denied(to)

    // A tab moving in the hash is not a new screen: nothing refetches.
    if (from.matched.length && to.path === from.path && JSON.stringify(to.query) === JSON.stringify(from.query)) {
      return
    }

    const previous = from.matched.length ? toRoute(from) : undefined
    handlers.forEach(handler => handler(toRoute(to), previous))
  })

  active.value = router
  return router
}

export const activeRouter = () => active.value!

export const onRouteChanged = (handler: RouteChangedHandler) => {
  handlers.add(handler)
  return () => handlers.delete(handler)
}

/** From when the signed-in user is known: guards apply to this route and every one after. */
export const startGuarding = () => {
  guarding = true
  notFound.value = denied(activeRouter().currentRoute.value)
}

/** For specs: forget the guards and the handlers. */
export const resetRouting = () => {
  guarding = false
  notFound.value = false
  handlers.clear()
}

const Router = {
  /** Goes to a path within the app (or back and forth through history, with a number). */
  go(path: string | number, reload = false) {
    if (typeof path === 'number') {
      activeRouter().go(path)
      return
    }

    activeRouter().push(toClientPath(path))
    reload && forceReloadWindow()
  },

  /** The address of a named route. */
  url(name: RouteName, params: Record<string, unknown> = {}) {
    return activeRouter().resolve({ name, params: params as Record<string, string> }).href
  },
}

export default Router

/** Links to the app's own screens stay in the app; the router takes them. */
const interceptLinkClick = (event: MouseEvent) => {
  const router = active.value

  if (
    !router ||
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  ) {
    return
  }

  const link = (event.target as Element | null)?.closest('a')

  if (!link || (link.target && link.target !== '_self') || link.hasAttribute('download')) {
    return
  }

  const url = new URL(link.href, location.href)

  if (url.origin !== location.origin || !url.pathname.startsWith(basePath())) {
    return
  }

  const path = toClientPath(`${url.pathname}${url.search}${url.hash}`)

  if (!router.resolve(path).matched.some(record => record.meta.screen && record.meta.screen !== '404')) {
    return
  }

  event.preventDefault()
  router.push(path)
}

addEventListener('click', interceptLinkClick)
