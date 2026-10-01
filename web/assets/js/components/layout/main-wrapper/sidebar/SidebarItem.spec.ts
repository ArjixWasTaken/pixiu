import { describe, expect, it, vi } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import Component from './SidebarItem.vue'

describe('sidebarItem', () => {
  const h = createHarness()

  const renderComponent = (props: Record<string, unknown> = {}) => {
    return h.render(Component, {
      props: {
        href: '#',
        ...props,
      },
      slots: {
        default: 'Home',
      },
    })
  }

  it('renders', () => expect(renderComponent().html()).toMatchSnapshot())

  it('navigates and toggles sidebar on single click', async () => {
    const mock = h.mock(eventBus, 'emit')
    renderComponent()

    await h.user.click(screen.getByText('Home'))

    await waitFor(
      () => {
        expect(mock).toHaveBeenCalledWith('TOGGLE_SIDEBAR')
      },
      { timeout: 500 },
    )
  })

  it('navigates at once when nothing listens for a double click', async () => {
    const mock = h.mock(eventBus, 'emit')
    renderComponent()

    await h.user.click(screen.getByText('Home'))

    expect(mock).toHaveBeenCalledWith('TOGGLE_SIDEBAR')
  })

  it('handles a double click instead of navigating when something listens for it', async () => {
    const mock = h.mock(eventBus, 'emit')
    const onDblclick = vi.fn()
    renderComponent({ onDblclick })

    await h.user.dblClick(screen.getByText('Home'))
    await new Promise(resolve => setTimeout(resolve, 200))

    expect(onDblclick).toHaveBeenCalled()
    expect(mock).not.toHaveBeenCalled()
  })
})
