import { reactive } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import isMobile from 'ismobilejs'
import { createHarness } from '@/__tests__/TestHarness'
import { cache } from '@/services/cache'
import { http } from '@/services/http'
import type { SongUpdateResult } from '@/stores/playableStore'
import { playableStore } from '@/stores/playableStore'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { overviewStore } from '@/stores/overviewStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { playlistStore } from '@/stores/playlistStore'
import { subsonic } from '@/services/subsonic'

describe('playableStore', () => {
  const h = createHarness({
    afterEach: () => {
      isMobile.any = false
      preferenceStore.temporary.transcode_on_mobile = false
      playlistStore.state.playlists = []
    },
  })

  it('counts a play, and notes when it was', async () => {
    const scrobble = h.mock(subsonic, 'scrobble').mockResolvedValue({})
    const song = h.factory('song').make({ play_count: 2, played_at: null })

    await playableStore.registerPlay(song)

    expect(scrobble).toHaveBeenCalledWith(song.id, true, undefined)
    expect(song.play_count).toBe(3)
    expect(Date.now() - new Date(song.played_at!).getTime()).toBeLessThan(5_000)
  })

  it('gets a song by ID', () => {
    const song = reactive(h.factory('song').make({ id: 'foo' }))
    playableStore.vault.set('foo', reactive(song))
    playableStore.vault.set('bar', reactive(h.factory('song').make({ id: 'bar' })))

    expect(playableStore.byId('foo')).toBe(song)
  })

  it('gets songs by IDs', () => {
    const foo = reactive(h.factory('song').make({ id: 'foo' }))
    const bar = reactive(h.factory('song').make({ id: 'bar' }))
    playableStore.vault.set('foo', foo)
    playableStore.vault.set('bar', bar)
    playableStore.vault.set('baz', reactive(h.factory('song').make({ id: 'baz' })))

    expect(playableStore.byIds(['foo', 'bar'])).toEqual([foo, bar])
  })

  it('gets formatted length', () => {
    expect(playableStore.getFormattedLength(h.factory('song').make({ length: 123 }))).toBe('2 min 3 sec')
    expect(
      playableStore.getFormattedLength([
        h.factory('song').make({ length: 122 }),
        h.factory('song').make({ length: 123 }),
      ]),
    ).toBe('4 min 5 sec')
  })

  it('gets songs by album', () => {
    const songs = reactive(h.factory('song').make({ album_id: 'iv' }, 2))
    playableStore.vault.set(songs[0].id, songs[0])
    playableStore.vault.set(songs[1].id, songs[1])
    const album = h.factory('album').make({ id: 'iv' })

    expect(playableStore.byAlbum(album)).toEqual(songs)
  })

  it('matches a song by title', () => {
    const song = h.factory('song').make({ title: 'An amazing song' })
    const songs = [song, ...h.factory('song').make(3)]

    expect(playableStore.matchSongsByTitle('An amazing song', songs)).toEqual(song)
    expect(playableStore.matchSongsByTitle('An Amazing Song', songs)).toEqual(song)
    expect(playableStore.matchSongsByTitle('Nonexistent song', songs)).toBeNull()
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

    const syncSongsMock = h.mock(playableStore, 'syncWithVault')
    const syncAlbumsMock = h.mock(albumStore, 'syncWithVault')
    const syncArtistsMock = h.mock(artistStore, 'syncWithVault')
    const removeAlbumsMock = h.mock(albumStore, 'removeByIds')
    const removeArtistsMock = h.mock(artistStore, 'removeByIds')
    const putMock = h.mock(http, 'put').mockResolvedValueOnce(result)

    await playableStore.updateSongs(songs, {
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

    expect(syncSongsMock).toHaveBeenCalledWith(result.songs)
    expect(syncAlbumsMock).toHaveBeenCalledWith(result.albums)
    expect(syncArtistsMock).toHaveBeenCalledWith(result.artists)
    expect(removeAlbumsMock).toHaveBeenCalledWith(['iv'])
    expect(removeArtistsMock).toHaveBeenCalledWith(['led-zeppelin'])
  })

  it('gets shareable URL', () => {
    const song = h.factory('song').make()
    expect(playableStore.getShareableUrl(song)).toBe(`http://test/#/songs/${song.id}`)
  })

  it('syncs new songs into the vault and applies playback state defaults', () => {
    const song = h.factory('song').make({
      playback_state: null,
    })

    const [synced] = playableStore.syncWithVault(song)

    expect(playableStore.vault.has(song.id)).toBe(true)
    expect(synced.playback_state).toBe('Stopped')

    // re-syncing the same song reuses the existing reactive entry
    const [resynced] = playableStore.syncWithVault(song)
    expect(resynced).toBe(synced)
  })

  it('refreshes play stats when a vaulted song play count changes', async () => {
    const refreshMock = h.mock(overviewStore, 'refreshPlayStats')

    const [synced] = playableStore.syncWithVault(h.factory('song').make({ play_count: 98 }))
    synced.play_count = 100

    await h.tick()
    expect(refreshMock).toHaveBeenCalledTimes(1)

    // re-syncing the same song does not double up the watcher
    playableStore.syncWithVault({ ...synced, play_count: 101 } as Song)
    synced.play_count = 102

    await h.tick()
    expect(refreshMock).toHaveBeenCalledTimes(2)
  })

  it('invalidates the album and artist song caches for a song', () => {
    const song = h.factory('song').make({ album_id: 'album-1', artist_id: 'artist-1' })
    const removeMock = h.mock(cache, 'remove')

    playableStore.invalidateAlbumAndArtistSongCaches(song)

    expect(removeMock).toHaveBeenCalledWith(['album.songs', 'album-1'])
    expect(removeMock).toHaveBeenCalledWith(['artist.songs', 'artist-1'])
  })

  it('fetches for playlist with cache', async () => {
    const songs = h.factory('song').make(3)
    const playlist = h.factory('playlist').make()
    h.mock(playlistStore, 'byId').mockReturnValueOnce(playlist)
    cache.set(['playlist.songs', playlist.id], songs)

    const getMock = h.mock(http, 'get')

    const fetched = await playableStore.fetchForPlaylist(playlist)

    expect(getMock).not.toHaveBeenCalled()
    expect(fetched).toEqual(songs)
    expect(playlist.playables).toEqual(songs)
  })

  it('fetches and deduplicates songs for playlists', async () => {
    const playlists = h.factory('playlist').make(3)
    const sharedSong = h.factory('song').make()
    const firstSong = h.factory('song').make()
    const secondSong = h.factory('song').make()
    const thirdSong = h.factory('song').make()
    const fetchMock = h
      .mock(playableStore, 'fetchForPlaylist')
      .mockResolvedValueOnce([sharedSong, firstSong])
      .mockResolvedValueOnce([secondSong, sharedSong])
      .mockResolvedValueOnce([thirdSong])

    const songs = await playableStore.fetchForPlaylists(playlists)

    expect(fetchMock).toHaveBeenNthCalledWith(1, playlists[0])
    expect(fetchMock).toHaveBeenNthCalledWith(2, playlists[1])
    expect(fetchMock).toHaveBeenNthCalledWith(3, playlists[2])
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(songs).toEqual([sharedSong, firstSong, secondSong, thirdSong])
  })

  it('syncs album properties', () => {
    const album = h.factory('album').make()
    const songs = h.factory('song').make(
      {
        album_id: album.id,
      },
      3,
    )

    playableStore.syncWithVault(songs)

    album.name = 'New Album Name'
    album.cover = 'https://test/new-album-cover.jpg'

    playableStore.syncAlbumProperties(album)

    playableStore.byIds<Song>(songs.map(song => song.id)).forEach(song => {
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

    playableStore.syncWithVault([...songsFromArtist, ...songsContributedByArtist])

    artist.name = 'New Artist Name'
    playableStore.syncArtistProperties(artist)

    songsFromArtist.forEach(({ artist_name }) => expect(artist_name).toBe('New Artist Name'))
    songsContributedByArtist.forEach(({ album_artist_name }) => expect(album_artist_name).toBe('New Artist Name'))
  })
})
