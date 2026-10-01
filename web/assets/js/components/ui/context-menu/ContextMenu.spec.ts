import { describe, expect, it, vi } from 'vite-plus/test'
import * as floating from '@floating-ui/dom'
import { waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { shallowRef } from 'vue'
import { ContextMenuKey } from '@/config/symbols'
import { logger } from '@/utils/logger'
import Component from './ContextMenu.vue'

// On a desktop: phones show menus as bottom sheets, unplaced.
vi.mock('@/composables/useViewport', async () => {
  const { ref } = await import('vue')
  return { useViewport: () => ({ isMobile: ref(false), isWide: ref(false) }) }
})

// Placement runs as usual; the spec looks at how it was asked for.
vi.mock('@floating-ui/dom', async importOriginal => {
  const original = await importOriginal<typeof import('@floating-ui/dom')>()
  return { ...original, computePosition: vi.fn(original.computePosition) }
})

describe('contextMenu', () => {
  const h = createHarness()

  const provide = (options: ReturnType<typeof shallowRef>) => ({
    global: {
      provide: {
        [ContextMenuKey as symbol]: options,
      },
    },
  })

  it('renders the popover root', () => {
    const { container } = h.render(Component, provide(shallowRef({ component: null, position: { top: 0, left: 0 } })))

    const root = container.querySelector<HTMLElement>('.context-menu[popover]')!
    expect(root).toBeTruthy()
    expect(root.getAttribute('popover')).toBe('manual')
    expect(root.getAttribute('role')).toBe('menu')
  })

  it('opens when options.component is set', async () => {
    const showSpy = vi.spyOn(HTMLElement.prototype, 'showPopover')
    const options = shallowRef<any>({
      component: null,
      position: { top: 0, left: 0 },
    })

    h.render(Component, provide(options))

    options.value = {
      component: { template: '<div>Menu Content</div>' },
      position: { top: 100, left: 200 },
    }

    await h.tick(2)

    expect(showSpy).toHaveBeenCalled()
    showSpy.mockRestore()
  })

  it('closes when options.component is cleared', async () => {
    const showSpy = vi.spyOn(HTMLElement.prototype, 'showPopover')
    const hideSpy = vi.spyOn(HTMLElement.prototype, 'hidePopover')
    const options = shallowRef<any>({
      component: null,
      position: { top: 0, left: 0 },
    })

    h.render(Component, provide(options))

    // Open the menu first so that close can transition from open → closed.
    options.value = {
      component: { template: '<div>Menu</div>' },
      position: { top: 100, left: 200 },
    }

    await h.tick(2)

    options.value = {
      component: null,
      position: { top: 0, left: 0 },
    }

    await h.tick()

    expect(hideSpy).toHaveBeenCalled()
    showSpy.mockRestore()
    hideSpy.mockRestore()
  })

  it('closes cleanly while it is still being placed', async () => {
    const errorSpy = vi.spyOn(logger, 'error')
    let place: (position: floating.ComputePositionReturn) => void = () => {}
    vi.mocked(floating.computePosition)
      .mockClear()
      .mockImplementationOnce(() => new Promise(resolve => (place = resolve)))
    const options = shallowRef<any>({ component: null, position: { top: 0, left: 0 } })
    const { unmount } = h.render(Component, provide(options))

    options.value = { component: { template: '<div>Menu</div>' }, position: { top: 100, left: 200 } }
    await waitFor(() => expect(floating.computePosition).toHaveBeenCalled())
    unmount()
    place({ x: 10, y: 20, placement: 'bottom-start', strategy: 'fixed', middlewareData: {} })
    // Everything the placement had left to do.
    await new Promise(resolve => setTimeout(resolve, 0))

    expect(errorSpy).not.toHaveBeenCalled()
  })

  it('applies extra class', () => {
    const { container } = h.render(Component, {
      props: { extraClass: 'my-custom-class' },
      ...provide(shallowRef({ component: null, position: { top: 0, left: 0 } })),
    })

    expect(container.querySelector('.my-custom-class[popover]')).toBeTruthy()
  })

  it('slides a menu too tall for its spot up or down, to keep it whole', async () => {
    const place = vi.mocked(floating.computePosition)
    place.mockClear()
    const options = shallowRef<any>({ component: null, position: { top: 0, left: 0 } })
    h.render(Component, provide(options))

    options.value = { component: { template: '<div>Menu</div>' }, position: { top: 880, left: 200 } }
    await waitFor(() => expect(place).toHaveBeenCalled())

    const middleware = place.mock.calls[0][2]!.middleware!.filter(Boolean) as floating.Middleware[]
    const shift = middleware.find(({ name }) => name === 'shift')!
    expect(shift.options).toMatchObject({ crossAxis: true })
  })
})
