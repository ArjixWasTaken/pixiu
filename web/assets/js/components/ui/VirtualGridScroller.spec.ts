import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { h as vnode } from 'vue'
import Component from './VirtualGridScroller.vue'

describe('virtualGridScroller', () => {
  const h = createHarness()

  const createItems = (count: number) => Array.from({ length: count }, (_, i) => ({ id: `id-${i}`, name: `Item ${i}` }))

  /** jsdom lays nothing out: an 800×600 grid of 200px-tall cards, as a browser would give it. */
  const layOut = () => {
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) {
      return this.classList.contains('virtual-grid-scroller') ? 800 : 0
    })

    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
      if (this.dataset.testid === 'grid-item') {
        return 200
      }

      return this.classList.contains('virtual-grid-scroller') ? 600 : 0
    })
  }

  const renderGrid = async (count: number) => {
    const rendered = h.render(Component, {
      props: { items: createItems(count), minItemWidth: 200 },
      slots: {
        default: ({ item }: { item: { name: string } }) => vnode('div', { 'data-testid': 'grid-item' }, item.name),
      },
    })

    await h.tick(3)

    return rendered
  }

  it('does not crash with empty items', async () => {
    h.render(Component, {
      props: { items: [], minItemWidth: 200 },
    })

    await h.tick(2)
    expect(document.body.innerHTML).toBeTruthy()
  })

  it('renders the rows in view, and a few around, as many to a row as fit', async () => {
    layOut()
    await renderGrid(100)

    // Four to a row; three rows in view, three more below.
    expect(screen.getAllByTestId('grid-item')).toHaveLength(24)
    screen.getByText('Item 23')
    expect(screen.queryByText('Item 24')).toBeNull()
  })

  it('says when the end is near', async () => {
    layOut()
    const { emitted } = await renderGrid(8)

    expect(emitted('scrolled-to-end')).toHaveLength(1)
  })
})
