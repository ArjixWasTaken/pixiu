import { describe, expect, it, vi } from 'vite-plus/test'
import { defineComponent, nextTick, ref, h as vnode } from 'vue'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './VirtualScroller.vue'

describe('virtualScroller.vue', () => {
  const h = createHarness()

  const makeItems = (count: number) => Array.from({ length: count }, (_, i) => ({ id: i, name: `Item ${i}` }))

  /**
   * jsdom lays nothing out: heights as a browser would give them. Rows are
   * 64px (or `rowHeight` says otherwise); what scrolls is 320px tall.
   */
  const layOut = (rowHeight: (index: number) => number = () => 64) =>
    vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
      if (this.dataset.index) {
        return rowHeight(Number(this.dataset.index))
      }

      return this.matches('.screen-body, .virtual-scroller:not(.nested)') ? 320 : 0
    })

  const renderList = async (items: { name: string }[]) => {
    const rendered = h.render(Component, {
      props: { items, itemHeight: 64 },
      slots: { default: ({ item }: { item: { name: string } }) => vnode('span', item.name) },
    })

    await nextTick()

    return { ...rendered, list: rendered.container.querySelector<HTMLElement>('.virtual-scroller')! }
  }

  it('renders the rows in view, and a few around', async () => {
    layOut()
    await renderList(makeItems(100))

    // Five in view, five more below.
    screen.getByText('Item 0')
    screen.getByText('Item 9')
    expect(screen.queryByText('Item 10')).toBeNull()
  })

  it('measures each row', async () => {
    // The first row is taller (a disc label above it): 100px, not 64.
    layOut(index => (index === 0 ? 100 : 64))

    const { list } = await renderList(makeItems(100))
    await nextTick()

    expect(list.firstElementChild!.getAttribute('style')).toContain(`height: ${100 + 99 * 64}px`)
  })

  it('scrolls to an item', async () => {
    layOut()
    const scroller = ref<InstanceType<typeof Component>>()
    const items = makeItems(100)
    const { container } = h.render(
      defineComponent({ setup: () => () => vnode(Component, { ref: scroller, items, itemHeight: 64 }) }),
    )
    await nextTick()
    const list = container.querySelector<HTMLElement>('.virtual-scroller')!
    list.scrollTo = vi.fn()

    scroller.value!.scrollToIndex(50)

    expect(list.scrollTo).toHaveBeenCalledWith(expect.objectContaining({ behavior: 'smooth' }))
  })

  it('says when the end is near', async () => {
    layOut()
    const { emitted } = await renderList(makeItems(3))

    expect(emitted('scrolled-to-end')).toHaveLength(1)
  })

  it('scrolls with the screen it is on', async () => {
    const items = makeItems(100)

    // A screen whose header takes the first 100px, above the list.
    const Screen = defineComponent({
      setup: () => () =>
        vnode('main', { class: 'screen-body' }, [
          vnode(
            Component,
            { items, itemHeight: 64 },
            { default: ({ item }: { item: { name: string } }) => vnode('span', item.name) },
          ),
        ]),
    })

    layOut()
    let scrollTop = 0
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
      return { top: this.classList.contains('virtual-scroller') ? 100 - scrollTop : 0 } as DOMRect
    })

    const { container } = h.render(Screen)
    const body = container.querySelector<HTMLElement>('.screen-body')!
    Object.defineProperty(body, 'scrollTop', { get: () => scrollTop })

    await nextTick()
    expect(container.querySelector('.virtual-scroller')!.classList.contains('nested')).toBe(true)

    scrollTop = 100 + 50 * 64
    body.dispatchEvent(new Event('scroll'))
    await nextTick()

    screen.getByText('Item 45')
    screen.getByText('Item 50')
    expect(screen.queryByText('Item 0')).toBeNull()
  })
})
