import { describe, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './SidebarYourLibrarySection.vue'

describe('sidebarYourLibrarySection.vue', () => {
  const h = createHarness()

  it('links the library', () => {
    h.render(Component)

    ;['All songs', 'Albums', 'Artists', 'Genres'].forEach(label =>
      screen.getByRole('link', { name: new RegExp(label) }),
    )
  })
})
