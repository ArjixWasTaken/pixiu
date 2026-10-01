import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'
import Component from './ArtistScreen.vue'

vi.mock('@/composables/useContextMenu')

describe('artistScreen.vue', () => {
  const h = createHarness()

  const renderComponent = async (path: string) => {
    const artist = h.factory('artist').make({ id: 'ar-1', name: 'Kevin MacLeod' })
    h.mock(artistStore, 'resolve').mockResolvedValue(artist)
    h.mock(albumStore, 'fetchForArtist').mockResolvedValue(h.factory('album').make(2))
    const fetchSongs = h.mock(playableStore, 'fetchSongsForArtist').mockResolvedValue(h.factory('song').make(3))

    h.visit(path)
    h.render(Component, { global: { stubs: { AlbumCard: h.stub('album-card') } } })

    await waitFor(() => expect(fetchSongs).toHaveBeenCalledWith('ar-1'))
    await screen.findByText('Kevin MacLeod')
    // The header's async parts, loaded before the test ends.
    await screen.findAllByRole('button', { name: /favorites$/ })
  }

  it('filters the songs on the Songs tab', async () => {
    await renderComponent('/artists/ar-1')

    await screen.findByRole('button', { name: 'Filter' })
  })

  it('offers no song filter on the other tabs', async () => {
    await renderComponent('/artists/ar-1?tab=albums')

    await screen.findByRole('button', { name: 'Shuffle' })
    expect(screen.queryByRole('button', { name: 'Filter' })).toBeNull()
  })
})
