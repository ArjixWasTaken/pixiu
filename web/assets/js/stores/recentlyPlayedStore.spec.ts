import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { library } from '@/services/library'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
describe('recentlyPlayedStore', () => {
  const h = createHarness()

  it('fetches when attempting to add a new song and the state is empty', async () => {
    useRecentlyPlayedStore().state.playables = []
    const songs = h.factory('song').make(3)
    const fetchMock = h.mock(library, 'recentlyPlayed').mockResolvedValue(songs)
    const added = h.factory('song').make()

    await useRecentlyPlayedStore().add(added)

    expect(fetchMock).toHaveBeenCalled()
    expect(useRecentlyPlayedStore().state.playables.map(({ id }) => id)).toEqual([
      added.id,
      ...songs.map(({ id }) => id),
    ])
  })

  it('adds a song to the state', async () => {
    const newSong = h.factory('song').make()
    const songs = h.factory('song').make(10)
    const exceptSongs = songs.slice(0, 6)

    // We don't want to keep the reference to the original songs
    useRecentlyPlayedStore().state.playables = JSON.parse(JSON.stringify(songs))
    useRecentlyPlayedStore().excerptState.playables = JSON.parse(JSON.stringify(exceptSongs))

    await useRecentlyPlayedStore().add(newSong)

    expect(useRecentlyPlayedStore().state.playables).toEqual([newSong, ...songs])
    expect(useRecentlyPlayedStore().excerptState.playables).toEqual([newSong, ...songs.slice(0, 5)])
  })

  it('deduplicates when adding a song to the state', async () => {
    const songs = h.factory('song').make(10)
    const newSong = songs[1]
    const exceptSongs = songs.slice(0, 6)

    // We don't want to keep the reference to the original songs
    useRecentlyPlayedStore().state.playables = JSON.parse(JSON.stringify(songs))
    useRecentlyPlayedStore().excerptState.playables = JSON.parse(JSON.stringify(exceptSongs))

    await useRecentlyPlayedStore().add(newSong)

    expect(useRecentlyPlayedStore().state.playables).toEqual([newSong, songs[0], ...songs.slice(2)])
    expect(useRecentlyPlayedStore().excerptState.playables).toEqual([newSong, songs[0], ...songs.slice(2, 6)])
  })
})
