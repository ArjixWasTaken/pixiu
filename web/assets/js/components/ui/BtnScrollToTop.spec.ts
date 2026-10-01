import { describe, expect, it, vi } from 'vite-plus/test'
import { defineComponent, h as vnode } from 'vue'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'

import Component from './BtnScrollToTop.vue'

describe('btnScrollToTop.vue', () => {
  const h = createHarness()

  // On a screen, which is what scrolls.
  const Screen = defineComponent({ setup: () => () => vnode('main', { class: 'screen-body' }, [vnode(Component)]) })

  it('renders', () => expect(h.render(Component).html()).toMatchSnapshot())

  it('scrolls the screen back to the top', async () => {
    const { container } = h.render(Screen)
    const body = container.querySelector<HTMLElement>('.screen-body')!
    body.scrollTo = vi.fn()

    await h.user.click(screen.getByTitle('Scroll to top', { hidden: true } as never))

    expect(body.scrollTo).toHaveBeenCalledWith({ top: 0, behavior: 'smooth' })
  })
})
