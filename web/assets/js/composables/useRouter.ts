import { getCurrentInstance, onUnmounted } from 'vue'
import type { Route } from '@/router'
import Router, { activeRouter, notFound, onRouteChanged as subscribe, startGuarding, toRoute } from '@/router'
import { toClientPath } from '@/utils/clientUrl'

/**
 * What the screens need of the router: where they are, ways elsewhere, and
 * word when the route changes. vue-router does the work (see `router.ts`).
 */
export const useRouter = () => {
  const current = () => activeRouter().currentRoute.value

  const getCurrentScreen = (): ScreenName => (notFound.value ? '404' : (current().meta.screen ?? '404'))
  const isCurrentScreen = (...screens: ScreenName[]) => screens.includes(getCurrentScreen())

  /** A parameter of the path, else of the query. */
  const getRouteParam = <T = string>(name: string) => toRoute(current()).params[name] as T

  /** Calls `handler` on each new route (not on a hash change); in a component, until it unmounts. */
  const onRouteChanged = (handler: (route: Route, previous?: Route) => unknown) => {
    const stop = subscribe(handler)
    getCurrentInstance() && onUnmounted(stop)
    return stop
  }

  /** Runs `cb` now if `screen` shows, and each time it comes up again. */
  const onScreenActivated = (screen: ScreenName, cb: Closure) => {
    isCurrentScreen(screen) && cb()
    onRouteChanged(route => route.screen === screen && cb())
  }

  return {
    getRouteParam,
    getCurrentScreen,
    isCurrentScreen,
    onScreenActivated,
    onRouteChanged,
    startGuarding,
    triggerNotFound: () => (notFound.value = true),
    go: (...args: Parameters<typeof Router.go>) => Router.go(...args),
    /** Goes to a path in place of the current history entry. */
    replace: (path: string) => activeRouter().replace(toClientPath(path)),
    url: (...args: Parameters<typeof Router.url>) => Router.url(...args),
  }
}
