import type { Ref } from 'vue'
import { getCurrentScope, onScopeDispose, watch } from 'vue'
import type { Router } from 'vue-router'

/** What's open now that Back closes, the last opened last. */
const open: Array<() => void> = []

/** The history entry the router last settled on: a navigation elsewhere is Back or Forward. */
let settledAt: unknown

const position = () => (history.state as { position?: unknown } | null)?.position

/**
 * Lets Back (a phone's back gesture, the browser's button) close what's open
 * before it leaves the page, as apps do. A navigation through history while
 * something is open is turned down (the router stays where it was) and closes
 * the last thing opened instead. Nothing is added to the history.
 */
export const closeOnBack = (router: Router) => {
  settledAt = position()

  router.afterEach((_to, _from, failure) => failure || (settledAt = position()))

  router.beforeEach(() => {
    // A push or replace hasn't touched the history yet; Back and Forward have.
    const throughHistory = position() !== settledAt

    if (throughHistory && open.length) {
      open.at(-1)!()
      return false
    }

    return true
  })
}

/** While `isOpen`, Back runs `close` (rather than leaving the page). */
export const useBackToClose = (isOpen: Ref<boolean>, close: () => void) => {
  const closer = () => close()

  const forget = () => {
    const index = open.lastIndexOf(closer)
    index === -1 || open.splice(index, 1)
  }

  watch(
    isOpen,
    value => {
      forget()
      value && open.push(closer)
    },
    { immediate: true },
  )

  getCurrentScope() && onScopeDispose(forget)
}
