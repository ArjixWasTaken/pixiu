import { screen } from '@testing-library/vue'
import { defineComponent, nextTick, ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ScreenBase.vue'

describe('screenBase', () => {
  const h = createHarness()

  it('renders header and default slots', () => {
    h.render(Component, {
      slots: {
        header: 'Screen Header',
        default: 'Screen Content',
      },
    })

    screen.getByText('Screen Header')
    screen.getByText('Screen Content')
  })

  it('scrolls the header with the content', () => {
    const { container } = h.render(Component, {
      slots: {
        header: 'Screen Header',
        default: 'Screen Content',
      },
    })

    expect(container.querySelector('.screen-body')!.textContent).toContain('Screen Header')
  })

  it('comes back where it was after being kept away', async () => {
    const shown = ref(true)
    const { container } = h.render(
      defineComponent({
        components: { ScreenBase: Component },
        setup: () => ({ shown }),
        template: '<KeepAlive><ScreenBase v-if="shown">Screen Content</ScreenBase></KeepAlive>',
      }),
    )

    const body = container.querySelector<HTMLElement>('.screen-body')!
    // jsdom lays nothing out: scrollTop is a plain property here.
    let top = 0
    Object.defineProperty(body, 'scrollTop', { get: () => top, set: (value: number) => (top = value) })
    const scrolls: Event[] = []
    body.addEventListener('scroll', event => scrolls.push(event))

    await nextTick()
    top = 480
    body.dispatchEvent(new Event('scroll'))

    shown.value = false
    await nextTick()
    // The browser forgets where a detached screen was.
    top = 0
    shown.value = true
    await nextTick()

    expect(top).toBe(480)
    // Virtual lists hear about it, even had it not moved.
    expect(scrolls.length).toBeGreaterThan(1)
  })
})
