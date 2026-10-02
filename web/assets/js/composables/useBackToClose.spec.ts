import { describe, expect, it, vi } from 'vite-plus/test'
import { effectScope, nextTick, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { activeRouter } from '@/router'
import { useBackToClose } from '@/composables/useBackToClose'

describe('useBackToClose', () => {
  const h = createHarness()

  /** Back, as the browser does it: the history moves first, then the router hears of it. */
  const back = async (to: string) => {
    history.replaceState({ ...history.state, position: (history.state?.position ?? 0) - 1 }, '')
    await activeRouter().push(to)
  }

  it('closes what is open rather than leaving the page', async () => {
    await h.visit('/albums')
    const open = ref(true)
    const close = vi.fn(() => (open.value = false))
    const scope = effectScope()
    scope.run(() => useBackToClose(open, close))

    await back('/home')

    expect(close).toHaveBeenCalledOnce()
    expect(activeRouter().currentRoute.value.path).toBe('/albums')
    scope.stop()
  })

  it('lets Back through once nothing is open', async () => {
    await h.visit('/albums')
    const open = ref(false)
    const close = vi.fn()
    const scope = effectScope()
    scope.run(() => useBackToClose(open, close))
    await nextTick()

    await back('/home')

    expect(close).not.toHaveBeenCalled()
    expect(activeRouter().currentRoute.value.path).toBe('/home')
    scope.stop()
  })

  it('never stands in the way of going somewhere', async () => {
    await h.visit('/albums')
    const open = ref(true)
    const close = vi.fn()
    const scope = effectScope()
    scope.run(() => useBackToClose(open, close))

    // A link or a push: the history hasn't moved.
    await activeRouter().push('/home')

    expect(close).not.toHaveBeenCalled()
    expect(activeRouter().currentRoute.value.path).toBe('/home')
    scope.stop()
  })
})
