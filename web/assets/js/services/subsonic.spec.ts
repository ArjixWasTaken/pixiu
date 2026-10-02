import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { coverOfSize, isNotFound, subsonic, SubsonicError } from '@/services/subsonic'

describe('subsonic', () => {
  const h = createHarness()

  it('asks for covers at the size they are shown', () => {
    const cover = 'https://music.example/rest/getCoverArt?apiKey=k&v=1.16.1&id=al-3'
    const sized = new URL(coverOfSize(cover, 128))

    expect(sized.searchParams.get('size')).toBe('128')
    expect(sized.searchParams.get('id')).toBe('al-3')
    expect(new URL(coverOfSize(`${cover}&size=300`, 64)).searchParams.getAll('size')).toEqual(['64'])
    // Images from elsewhere stay as they are.
    expect(coverOfSize('https://lh3.googleusercontent.com/cover.jpg', 128)).toBe(
      'https://lh3.googleusercontent.com/cover.jpg',
    )
    expect(coverOfSize(null, 128)).toBe('')
  })

  it('maps a Subsonic song onto koel’s', () => {
    const song = subsonic.toSong({
      id: 'tr-3',
      title: 'Funky Chunk',
      duration: 239,
      playCount: 2,
      played: '2026-09-30T08:00:00Z',
      userRating: 4,
      starred: '2026-09-29T16:30:34Z',
      albumId: 'al-3',
      album: 'Groovy',
      coverArt: 'al-3',
      artistId: 'ar-1',
      artist: 'Kevin MacLeod',
      displayAlbumArtist: 'Kevin MacLeod',
      track: 1,
      discNumber: 1,
      year: 2016,
      path: 'Kevin MacLeod/2016 - Groovy/01 Funky Chunk.opus',
      sourcePlatform: 'youtube_music',
    })

    expect(song).toMatchObject({
      type: 'songs',
      id: 'tr-3',
      length: 239,
      play_count: 2,
      played_at: '2026-09-30T08:00:00Z',
      rating: 4,
      favorite: true,
      album_id: 'al-3',
      artist_id: 'ar-1',
      album_artist_id: 'ar-1',
      basename: '01 Funky Chunk.opus',
      source_platform: 'youtube_music',
    })
    expect(song.album_cover).toContain('/rest/getCoverArt?')
    expect(song.album_cover).toContain('id=al-3')
  })

  it('says where an album was downloaded from; uploads come from nowhere', () => {
    expect(subsonic.toAlbum({ id: 'al-3', name: 'Groovy', sourcePlatform: 'youtube_music' }).source_platform).toBe(
      'youtube_music',
    )
    expect(subsonic.toAlbum({ id: 'al-4', name: 'Uploaded' }).source_platform).toBeNull()
    expect(subsonic.toSong({ id: 'tr-4', title: 'Uploaded' }).source_platform).toBeNull()
  })

  it('counts an artist’s albums', () => {
    expect(subsonic.toArtist({ id: 'ar-1', name: 'Kevin MacLeod', albumCount: 3 })).toMatchObject({
      id: 'ar-1',
      name: 'Kevin MacLeod',
      album_count: 3,
    })
  })

  it('tells mirrors of watched playlists from smart playlists', () => {
    const mirror = subsonic.toPlaylist({ id: 'pl-1', name: 'Road trip', readonly: true })
    const smart = subsonic.toPlaylist({
      id: 'pl-2',
      name: 'Lights',
      readonly: true,
      rules: [{ id: 'g', rules: [] }],
      folderId: '3',
    })
    const own = subsonic.toPlaylist({ id: 'pl-3', name: 'Mine', readonly: false })

    expect(mirror.permissions).toEqual({ edit: false, delete: false })
    expect(smart).toMatchObject({ is_smart: true, folder_id: '3', permissions: { edit: true, delete: true } })
    expect(own).toMatchObject({ is_smart: false, folder_id: null, permissions: { edit: true, delete: true } })
  })

  it('turns synced lyrics into LRC', async () => {
    h.mock(globalThis, 'fetch').mockResolvedValue(
      new Response(
        JSON.stringify({
          'subsonic-response': {
            status: 'ok',
            lyricsList: {
              structuredLyrics: [
                { synced: false, line: [{ value: 'plain' }] },
                {
                  synced: true,
                  line: [
                    { start: 1000, value: 'First' },
                    { start: 75_250, value: 'Later' },
                  ],
                },
              ],
            },
          },
        }),
      ),
    )

    expect(await subsonic.lyrics('tr-1')).toBe('[00:01.00]First\n[01:15.25]Later')
  })

  it('builds stream URLs with the key, transcoding when asked', () => {
    expect(subsonic.streamUrl('tr-1')).toContain('/rest/stream?')
    expect(subsonic.streamUrl('tr-1')).not.toContain('maxBitRate')
    expect(subsonic.streamUrl('tr-1', 128)).toContain('maxBitRate=128')
  })

  it('knows a missing thing, from Subsonic or the JSON API', () => {
    expect(isNotFound(new SubsonicError(70, 'Album not found'))).toBe(true)
    expect(isNotFound({ status: 404 })).toBe(true)
    expect(isNotFound(new SubsonicError(10, 'Missing parameter'))).toBe(false)
    expect(isNotFound(new Error('offline'))).toBe(false)
  })
})
