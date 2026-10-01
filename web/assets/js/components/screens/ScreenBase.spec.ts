import { screen } from '@testing-library/vue'
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
})
