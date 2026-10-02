import { screen, waitFor, within } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { usePlayableStore } from '@/stores/playableStore'
import Component from './ArtistScreen.vue'

vi.mock('@/composables/useContextMenu')

describe('artistScreen.vue', () => {
  const h = createHarness()

  const renderComponent = async (
    path: string,
    songs = h.factory('song').make(3),
    albums = h.factory('album').make(2),
  ) => {
    const artist = h.factory('artist').make({ id: 'ar-1', name: 'Kevin MacLeod' })
    h.mock(useArtistStore(), 'resolve').mockResolvedValue(artist)
    h.mock(useAlbumStore(), 'fetchForArtist').mockResolvedValue(albums)
    const fetchSongs = h.mock(usePlayableStore(), 'fetchSongsForArtist').mockResolvedValue(songs)

    await h.visit(path)
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

  it('lists singles on their own, apart from the albums', async () => {
    const songs = [
      h.factory('song').make({ title: 'Overture', album_id: 'al-1', album_name: 'Suite' }),
      h.factory('song').make({ title: 'Dawn Chorus', album_id: 'al-2', album_name: 'Dawn Chorus', is_single: true }),
    ]
    const albums = [
      h.factory('album').make({ id: 'al-1', name: 'Suite' }),
      h.factory('album').make({ id: 'al-2', name: 'Dawn Chorus', is_single: true }),
    ]
    await renderComponent('/artists/ar-1#albums', songs, albums)

    screen.getByText('1 album')
    screen.getByText('1 single')
    await waitFor(() => expect(screen.getAllByTestId('album-card')).toHaveLength(1))

    await h.user.click(screen.getByRole('tab', { name: 'Singles' }))
    const singles = screen.getByRole('tabpanel', { name: 'Singles' })
    expect(within(singles).getAllByTestId('song-card')).toHaveLength(1)
    within(singles).getByText('Dawn Chorus')
  })

  it('has no Singles tab for an artist without singles', async () => {
    await renderComponent('/artists/ar-1')

    expect(screen.queryByRole('tab', { name: 'Singles' })).toBeNull()
  })

  it('offers no song filter on the other tabs', async () => {
    await renderComponent('/artists/ar-1#albums')

    await screen.findByRole('button', { name: 'Shuffle' })
    expect(screen.queryByRole('button', { name: 'Filter' })).toBeNull()
  })
})
