import { describe, expect, it } from 'vite-plus/test'
import { defineComponent } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useRouter } from './useRouter'

describe('useRouter', () => {
  const h = createHarness()

  it('reads a parameter of the path, else of the query', async () => {
    await h.visit('/genres/Rock?sort=name')

    const { getRouteParam } = useRouter()
    expect(getRouteParam('id')).toBe('Rock')
    expect(getRouteParam('sort')).toBe('name')
  })

  it('knows the screen showing', async () => {
    await h.visit('/albums')

    const { getCurrentScreen, isCurrentScreen } = useRouter()
    expect(getCurrentScreen()).toBe('Albums')
    expect(isCurrentScreen('Albums', 'Album')).toBe(true)
    expect(isCurrentScreen('Queue')).toBe(false)
  })

  it('runs a screen hook now, and when the screen comes up again', async () => {
    await h.visit('/albums')
    let runs = 0
    useRouter().onScreenActivated('Albums', () => runs++)
    expect(runs).toBe(1)

    await h.visit('/home')
    await h.visit('/albums')
    expect(runs).toBe(2)
  })

  it('stops telling a component of route changes once it unmounts', async () => {
    let calls = 0
    const Listener = defineComponent({
      setup: () => useRouter().onRouteChanged(() => calls++) && undefined,
      render: () => null,
    })

    const { unmount } = h.render(Listener)
    await h.visit('/albums')
    unmount()
    await h.visit('/artists')

    expect(calls).toBe(1)
  })
})
