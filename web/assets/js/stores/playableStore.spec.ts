import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { queryClient } from '@/services/queryClient'
import { http } from '@/services/http'
import type { SongUpdateResult } from '@/stores/playableStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { useOverviewStore } from '@/stores/overviewStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { subsonic } from '@/services/subsonic'

describe('playableStore', () => {
  const h = createHarness()

  it('counts a play, and notes when it was', async () => {
    const scrobble = h.mock(subsonic, 'scrobble').mockResolvedValue({})
    const song = h.factory('song').make({ play_count: 2, played_at: null })

    await usePlayableStore().registerPlay(song)

    expect(scrobble).toHaveBeenCalledWith(song.id, true, undefined)
    expect(song.play_count).toBe(3)
    expect(Date.now() - new Date(song.played_at!).getTime()).toBeLessThan(5_000)
  })

  it('gets a song by ID', () => {
    const song = reactive(h.factory('song').make({ id: 'foo' }))
    usePlayableStore().vault.set('foo', reactive(song))
    usePlayableStore().vault.set('bar', reactive(h.factory('song').make({ id: 'bar' })))

    expect(usePlayableStore().byId('foo')).toBe(song)
  })

  it('gets songs by IDs', () => {
    const foo = reactive(h.factory('song').make({ id: 'foo' }))
    const bar = reactive(h.factory('song').make({ id: 'bar' }))
    usePlayableStore().vault.set('foo', foo)
    usePlayableStore().vault.set('bar', bar)
    usePlayableStore().vault.set('baz', reactive(h.factory('song').make({ id: 'baz' })))

    expect(usePlayableStore().byIds(['foo', 'bar'])).toEqual([foo, bar])
  })

  it('gets formatted length', () => {
    expect(usePlayableStore().getFormattedLength(h.factory('song').make({ length: 123 }))).toBe('2 min 3 sec')
    expect(
      usePlayableStore().getFormattedLength([
        h.factory('song').make({ length: 122 }),
        h.factory('song').make({ length: 123 }),
      ]),
    ).toBe('4 min 5 sec')
  })

  it('gets songs by album', () => {
    const songs = reactive(h.factory('song').make({ album_id: 'iv' }, 2))
    usePlayableStore().vault.set(songs[0].id, songs[0])
    usePlayableStore().vault.set(songs[1].id, songs[1])
    const album = h.factory('album').make({ id: 'iv' })

    expect(usePlayableStore().byAlbum(album)).toEqual(songs)
  })

  it('matches a song by title', () => {
    const song = h.factory('song').make({ title: 'An amazing song' })
    const songs = [song, ...h.factory('song').make(3)]

    expect(usePlayableStore().matchSongsByTitle('An amazing song', songs)).toEqual(song)
    expect(usePlayableStore().matchSongsByTitle('An Amazing Song', songs)).toEqual(song)
    expect(usePlayableStore().matchSongsByTitle('Nonexistent song', songs)).toBeNull()
  })

  it('updates songs', async () => {
    const songs = h.factory('song').make(3)

    const result: SongUpdateResult = {
      songs: h.factory('song').make(3),
      albums: h.factory('album').make(2),
      artists: h.factory('artist').make(2),
      removed: {
        album_ids: ['iv'],
        artist_ids: ['led-zeppelin'],
      },
    }

    const syncAlbumsMock = h.mock(useAlbumStore(), 'syncWithVault')
    const syncArtistsMock = h.mock(useArtistStore(), 'syncWithVault')
    const removeAlbumsMock = h.mock(useAlbumStore(), 'removeByIds')
    const removeArtistsMock = h.mock(useArtistStore(), 'removeByIds')
    const putMock = h.mock(http, 'put').mockResolvedValueOnce(result)

    await usePlayableStore().updateSongs(songs, {
      album_name: 'Updated Album',
      artist_name: 'Updated Artist',
    })

    expect(putMock).toHaveBeenCalledWith('songs', {
      data: {
        album_name: 'Updated Album',
        artist_name: 'Updated Artist',
      },
      songs: songs.map(song => song.id),
    })

    result.songs.forEach(song => expect(usePlayableStore().byId(song.id)).toMatchObject({ id: song.id }))
    expect(syncAlbumsMock).toHaveBeenCalledWith(result.albums)
    expect(syncArtistsMock).toHaveBeenCalledWith(result.artists)
    expect(removeAlbumsMock).toHaveBeenCalledWith(['iv'])
    expect(removeArtistsMock).toHaveBeenCalledWith(['led-zeppelin'])
  })

  it('syncs new songs into the vault and applies playback state defaults', () => {
    const song = h.factory('song').make({
      playback_state: null,
    })

    const [synced] = usePlayableStore().syncWithVault(song)

    expect(usePlayableStore().vault.has(song.id)).toBe(true)
    expect(synced.playback_state).toBe('Stopped')

    // re-syncing the same song reuses the existing reactive entry
    const [resynced] = usePlayableStore().syncWithVault(song)
    expect(resynced).toBe(synced)
  })

  it('refreshes play stats when a vaulted song play count changes', async () => {
    const refreshMock = h.mock(useOverviewStore(), 'refreshPlayStats')

    const [synced] = usePlayableStore().syncWithVault(h.factory('song').make({ play_count: 98 }))
    synced.play_count = 100

    await h.tick()
    expect(refreshMock).toHaveBeenCalledTimes(1)

    // re-syncing the same song does not double up the watcher
    usePlayableStore().syncWithVault({ ...synced, play_count: 101 } as Song)
    synced.play_count = 102

    await h.tick()
    expect(refreshMock).toHaveBeenCalledTimes(2)
  })

  it('makes the song lists of its album and artist stale, for an edited song', async () => {
    const song = h.factory('song').make({ album_id: 'album-1', artist_id: 'artist-1' })
    queryClient.setQueryData(['album', 'album-1', 'songs'], [])
    queryClient.setQueryData(['artist', 'artist-1', 'songs'], [])

    await usePlayableStore().invalidateAlbumAndArtistSongCaches(song)

    expect(queryClient.getQueryState(['album', 'album-1', 'songs'])?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(['artist', 'artist-1', 'songs'])?.isInvalidated).toBe(true)
  })

  it('fetches the songs of a playlist once while they are fresh', async () => {
    const songs = h.factory('song').make(3)
    const playlist = h.factory('playlist').make()
    h.mock(usePlaylistStore(), 'byId').mockReturnValue(playlist)
    const fetchMock = h.mock(subsonic, 'playlistSongs').mockResolvedValue(songs)

    await usePlayableStore().fetchForPlaylist(playlist)
    const fetched = await usePlayableStore().fetchForPlaylist(playlist)

    expect(fetchMock).toHaveBeenCalledTimes(1)
    expect(fetched).toEqual(songs)
    expect(playlist.playables).toEqual(songs)
  })

  it('fetches and deduplicates songs for playlists', async () => {
    const playlists = h.factory('playlist').make(3)
    usePlaylistStore().state.playlists = playlists
    const [sharedSong, firstSong, secondSong, thirdSong] = h.factory('song').make(4)
    const fetchMock = h
      .mock(subsonic, 'playlistSongs')
      .mockResolvedValueOnce([sharedSong, firstSong])
      .mockResolvedValueOnce([secondSong, sharedSong])
      .mockResolvedValueOnce([thirdSong])

    const songs = await usePlayableStore().fetchForPlaylists(playlists)

    expect(fetchMock.mock.calls).toEqual([[playlists[0].id], [playlists[1].id], [playlists[2].id]])
    expect(songs.map(({ id }) => id)).toEqual([sharedSong.id, firstSong.id, secondSong.id, thirdSong.id])
  })

  it('syncs album properties', () => {
    const album = h.factory('album').make()
    const songs = h.factory('song').make(
      {
        album_id: album.id,
      },
      3,
    )

    usePlayableStore().syncWithVault(songs)

    album.name = 'New Album Name'
    album.cover = 'https://test/new-album-cover.jpg'

    usePlayableStore().syncAlbumProperties(album)

    usePlayableStore()
      .byIds<Song>(songs.map(song => song.id))
      .forEach(song => {
        expect(song.album_name).toBe('New Album Name')
        expect(song.album_cover).toBe('https://test/new-album-cover.jpg')
      })
  })

  it('syncs artist properties', () => {
    const artist = h.factory('artist').make()

    const songsFromArtist = h.factory('song').make(
      {
        artist_id: artist.id,
      },
      3,
    )

    const songsContributedByArtist = h.factory('song').make(
      {
        album_artist_id: artist.id,
      },
      2,
    )

    usePlayableStore().syncWithVault([...songsFromArtist, ...songsContributedByArtist])

    artist.name = 'New Artist Name'
    usePlayableStore().syncArtistProperties(artist)

    songsFromArtist.forEach(({ artist_name }) => expect(artist_name).toBe('New Artist Name'))
    songsContributedByArtist.forEach(({ album_artist_name }) => expect(album_artist_name).toBe('New Artist Name'))
  })
})
