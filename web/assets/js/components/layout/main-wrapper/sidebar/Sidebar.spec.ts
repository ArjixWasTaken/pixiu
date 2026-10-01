import { afterEach, describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import { setViewport } from '@/composables/useViewport'
import Component from './Sidebar.vue'

const standardItems = ['Home', 'All songs', 'Albums', 'Artists', 'Genres', 'Favorites', 'Recently played']

describe('sidebar.vue on desktop', () => {
  const h = createHarness({
    beforeEach: () => {
      localStorage.clear()
      setViewport({ mobile: false })
    },
  })

  afterEach(() => setViewport({ mobile: true, wide: true }))

  it('shows the standard items', () => {
    h.actingAsUser().render(Component)
    standardItems.forEach(label => screen.getByText(label))
  })

  it('collapses into a rail and expands again', async () => {
    h.render(Component)

    await h.user.click(screen.getByRole('button', { name: 'Collapse navigation' }))
    expect(screen.queryByTestId('sidebar')).toBeNull()
    // The rail lists every destination.
    screen.getByText('Genres')
    screen.getByText('Discover')

    await h.user.click(screen.getByRole('button', { name: 'Open navigation' }))
    screen.getByTestId('sidebar')
  })
})

describe('sidebar.vue on a phone', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: true }),
  })

  it('opens as a modal drawer and closes', async () => {
    h.render(Component)
    expect(screen.queryByTestId('sidebar')).toBeNull()

    eventBus.emit('TOGGLE_SIDEBAR')
    await h.tick()
    screen.getByTestId('sidebar')

    await h.user.click(screen.getByRole('button', { name: 'Close navigation' }))
    expect(screen.queryByTestId('sidebar')).toBeNull()
  })
})
