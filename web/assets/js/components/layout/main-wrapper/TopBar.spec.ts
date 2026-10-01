import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { setViewport } from '@/composables/useViewport'
import Component from './TopBar.vue'

describe('topBar.vue', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: false }),
  })

  const renderComponent = () =>
    h.render(Component, { global: { stubs: { ProfileDropdown: h.stub('profile-dropdown') } } })

  it('searches the library', async () => {
    await h.visit('/home')
    renderComponent()

    screen.getByRole('searchbox')
    screen.getByTestId('profile-dropdown')
  })

  it('leaves searching to Discover’s own field there', async () => {
    await h.visit('/discover')
    renderComponent()

    expect(screen.queryByRole('searchbox')).toBeNull()
    screen.getByTestId('profile-dropdown')
  })

  it('keeps the account menu on phones at Discover, where the search field that holds it is gone', async () => {
    setViewport({ mobile: true })
    await h.visit('/discover')
    renderComponent()

    screen.getByTestId('profile-dropdown')
  })
})
