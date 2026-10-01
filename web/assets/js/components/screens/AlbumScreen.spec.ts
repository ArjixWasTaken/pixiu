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
    const fetchForArtist = h.mock(albumStore, 'fetchForArtist').mockResolvedValue([album])
    const fetchSongs = h.mock(playableStore, 'fetchSongsForAlbum').mockResolvedValue(h.factory('song').make(3))

    await h.visit(path)
    h.render(Component, { global: { stubs: { AlbumCard: h.stub('album-card') } } })

    await waitFor(() => expect(fetchSongs).toHaveBeenCalledWith('al-1'))
    await screen.findByText('Groovy')
    // The header's async parts, loaded before the test ends.
    await screen.findAllByRole('button', { name: /favorites$/ })

    return { fetchForArtist }
  }

  it('filters the songs on the Songs tab', async () => {
    await renderComponent('/albums/al-1')

    await screen.findByRole('button', { name: 'Filter' })
  })

  it('offers no song filter on the other tabs', async () => {
    await renderComponent('/albums/al-1#other-albums')

    await screen.findByRole('button', { name: 'Shuffle' })
    expect(screen.queryByRole('button', { name: 'Filter' })).toBeNull()
  })

  it('opens the tab the hash names', async () => {
    const { fetchForArtist } = await renderComponent('/albums/al-1#other-albums')

    await waitFor(() => expect(fetchForArtist).toHaveBeenCalled())
    expect(screen.getByTestId('albums-pane').style.display).toBe('')
  })

  it('keeps the chosen tab in the hash, and fetches its albums then', async () => {
    const { fetchForArtist } = await renderComponent('/albums/al-1')
    expect(fetchForArtist).not.toHaveBeenCalled()

    await h.user.click(screen.getByRole('tab', { name: 'Other albums' }))

    await waitFor(() => expect(h.router.currentRoute.value.hash).toBe('#other-albums'))
    await waitFor(() => expect(fetchForArtist).toHaveBeenCalled())
  })

  it('takes a link from before, with the tab in the path, to the hash', async () => {
    await renderComponent('/albums/al-1/information')

    expect(h.router.currentRoute.value.fullPath).toBe('/albums/al-1#information')
  })
})
