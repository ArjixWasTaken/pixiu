import { describe, expect, it, vi } from 'vite-plus/test'
import { defineComponent, nextTick, h as vnode } from 'vue'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './VirtualScroller.vue'

describe('virtualScroller.vue', () => {
  const h = createHarness()

  it('renders items via scoped slot', () => {
    const items = [
      { id: 1, name: 'Item 1' },
      { id: 2, name: 'Item 2' },
    ]

    const { container } = h.render(Component, {
      props: { items, itemHeight: 40 },
      slots: {
        default: (props: { item: { name: string } }) => props.item.name,
      },
    })

    expect(container.querySelector('.virtual-scroller')).toBeTruthy()
  })

  it('exposes scrollToIndex that scrolls to the correct position', () => {
    const items = Array.from({ length: 100 }, (_, i) => ({ id: i, name: `Item ${i}` }))
    const itemHeight = 64

    const { container } = h.render(Component, {
      props: { items, itemHeight },
      slots: {
        default: (props: { item: { name: string } }) => props.item.name,
      },
    })

    const scrollerEl = container.querySelector('.virtual-scroller') as HTMLElement
    const scrollToMock = vi.fn()
    scrollerEl.scrollTo = scrollToMock

    const instance = (scrollerEl as any)['__vueParentComponent']
    instance?.exposed?.scrollToIndex(50)

    expect(scrollToMock).toHaveBeenCalledWith(expect.objectContaining({ behavior: 'smooth' }))
  })

  it('scrolls with the screen it is on', async () => {
    const items = Array.from({ length: 100 }, (_, i) => ({ id: i, name: `Item ${i}` }))

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

    const { container } = h.render(Screen)
    const body = container.querySelector<HTMLElement>('.screen-body')!
    let scrollTop = 0
    Object.defineProperty(body, 'scrollTop', { get: () => scrollTop })
    Object.defineProperty(body, 'scrollHeight', { get: () => 100 + 100 * 64 })

    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
      const top = this.classList.contains('virtual-scroller') ? 100 - scrollTop : 0
      return { top } as DOMRect
    })

    await nextTick()
    expect(container.querySelector('.virtual-scroller')!.classList.contains('nested')).toBe(true)

    scrollTop = 100 + 50 * 64
    body.dispatchEvent(new Event('scroll'))
    await new Promise(requestAnimationFrame)
    await nextTick()

    screen.getByText('Item 45')
    expect(screen.queryByText('Item 0')).toBeNull()
  })
})
