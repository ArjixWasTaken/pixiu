import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './TabList.vue'

describe('tabList.vue', () => {
  const h = createHarness()

  it('renders slot content with tablist role', () => {
    const { getByRole } = h.render(Component, {
      slots: { default: 'Tab items' },
    })

    expect(getByRole('tablist').textContent).toBe('Tab items')
  })

  it('scrolls the selected tab into view, so it is never hidden past an edge', () => {
    const revealed: Element[] = []
    const original = Element.prototype.scrollIntoView
    Element.prototype.scrollIntoView = vi.fn(function (this: Element) {
      revealed.push(this)
    })

    h.render(Component, {
      slots: {
        default:
          '<button role="tab" aria-selected="false">One</button><button role="tab" aria-selected="true">Two</button>',
      },
    })

    expect(revealed.map(tab => tab.textContent)).toEqual(['Two'])
    Element.prototype.scrollIntoView = original
  })

  it('sticks to the top when asked', () => {
    const { getByRole } = h.render(Component, { props: { sticky: true }, slots: { default: 'Tabs' } })

    expect(getByRole('tablist').classList.contains('sticky')).toBe(true)
  })
})
