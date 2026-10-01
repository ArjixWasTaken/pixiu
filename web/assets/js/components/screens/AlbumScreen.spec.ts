import { screen, waitFor } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { albumStore } from '@/stores/albumStore'
import { playableStore } from '@/stores/playableStore'
import Component from './AlbumScreen.vue'

vi.mock('@/composables/useContextMenu')

describe('albumScreen.vue', () => {
  const h = createHarness()

  const renderComponent = async (path: string) => {
    const album = h.factory('album').make({ id: 'al-1', name: 'Groovy' })
    h.mock(albumStore, 'resolve').mockResolvedValue(album)
    h.mock(albumStore, 'fetchForArtist').mockResolvedValue([album])
    const fetchSongs = h.mock(playableStore, 'fetchSongsForAlbum').mockResolvedValue(h.factory('song').make(3))

    h.visit(path)
    h.render(Component, { global: { stubs: { AlbumCard: h.stub('album-card') } } })

    await waitFor(() => expect(fetchSongs).toHaveBeenCalledWith('al-1'))
    await screen.findByText('Groovy')
    // The header's async parts, loaded before the test ends.
    await screen.findByRole('button', { name: /favorites$/ })
  }

  it('filters the songs on the Songs tab', async () => {
    await renderComponent('/albums/al-1')

    await screen.findByRole('button', { name: 'Filter' })
  })

  it('offers no song filter on the other tabs', async () => {
    await renderComponent('/albums/al-1?tab=other-albums')

    await screen.findByRole('button', { name: 'Shuffle' })
    expect(screen.queryByRole('button', { name: 'Filter' })).toBeNull()
  })
})
