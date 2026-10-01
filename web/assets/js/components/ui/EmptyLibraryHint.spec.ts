import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './EmptyLibraryHint.vue'

describe('emptyLibraryHint.vue', () => {
  const h = createHarness()

  it('points someone who can add music to uploads and Discover', () => {
    h.actingAsAdmin().render(Component)

    screen.getByRole('link', { name: 'Upload some music' })
    screen.getByRole('link', { name: 'Discover' })
  })

  it('says nothing to someone who cannot add music', () => {
    h.actingAsUser(h.factory('user').make({ abilities: [] }) as CurrentUser).render(Component)

    expect(screen.queryByText('Upload some music')).toBeNull()
  })
})
