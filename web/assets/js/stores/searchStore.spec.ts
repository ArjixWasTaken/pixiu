import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { subsonic } from '@/services/subsonic'
import { useSearchStore } from '@/stores/searchStore'

describe('searchStore', () => {
  // Each spec has stores of its own, fresh.
  const h = createHarness()

  it('resets the song result state', () => {
    useSearchStore().state.playables = h.factory('song').make(3)
    useSearchStore().resetPlayableResultState()
    expect(useSearchStore().state.playables).toEqual([])
  })

  it('leaves singles out of the albums found: their songs are among the songs', async () => {
    const singles = h
      .factory('album')
      .make(5)
      .map(album => ({ ...album, is_single: true }))
    const albums = h.factory('album').make(8)
    const search = h
      .mock(subsonic, 'search')
      .mockResolvedValue({ songs: [], albums: [...singles, ...albums], artists: [] })

    const found = await useSearchStore().excerptSearch('dawn')

    expect(search).toHaveBeenCalledWith('dawn', 6, 24)
    expect(found.albums.map(album => album.id)).toEqual(albums.slice(0, 6).map(album => album.id))
  })
})
