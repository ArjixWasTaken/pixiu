/**
 * píxiū's Subsonic API, as koel's stores need it: calls authenticated with the
 * web session's API key, and conversions of Subsonic's JSON into koel's types.
 */
import { authService } from '@/services/authService'
import { eventBus } from '@/utils/eventBus'

type Param = string | number | boolean | null | undefined
type Params = Record<string, Param | Param[]>

/** Subsonic error codes that mean the key is no good (OpenSubsonic 40, 44). */
const AUTH_ERRORS = [40, 44]

export class SubsonicError extends Error {
  constructor(
    public readonly code: number,
    message: string,
  ) {
    super(message)
  }
}

/** The URL of a Subsonic method, credentials included (for `<audio>`, `<img>`). */
const url = (method: string, params: Params = {}) => {
  const target = new URL(`${window.KOEL.base_url}rest/${method}`, window.location.origin)
  target.searchParams.set('apiKey', authService.getApiToken() ?? '')
  target.searchParams.set('v', '1.16.1')
  target.searchParams.set('c', 'pixiu-web')
  target.searchParams.set('f', 'json')

  for (const [name, value] of Object.entries(params)) {
    for (const item of Array.isArray(value) ? value : [value]) {
      if (item !== null && item !== undefined) {
        target.searchParams.append(name, String(item))
      }
    }
  }

  return target.toString()
}

/**
 * Calls a Subsonic method. `post` sends the parameters as a form instead of
 * in the URL, for long lists of ids.
 */
const call = async <T = Record<string, any>>(method: string, params: Params = {}, post = false): Promise<T> => {
  let response: Response

  if (post) {
    const form = new URLSearchParams()

    for (const [name, value] of Object.entries(params)) {
      for (const item of Array.isArray(value) ? value : [value]) {
        if (item !== null && item !== undefined) {
          form.append(name, String(item))
        }
      }
    }

    response = await fetch(url(method), { method: 'POST', body: form })
  } else {
    response = await fetch(url(method, params))
  }

  const body = (await response.json())['subsonic-response']

  if (body.status !== 'ok') {
    const code = body.error?.code ?? 0

    if (AUTH_ERRORS.includes(code)) {
      authService.setRedirect()
      eventBus.emit('LOG_OUT')
    }

    throw new SubsonicError(code, body.error?.message ?? `${method} failed`)
  }

  return body as T
}

/** A cover (album `al-…`, artist `ar-…`, playlist `pl-…`) at a given size. */
const coverUrl = (id: string | null | undefined, size?: number) => (id ? url('getCoverArt', { id, size }) : '')

const basename = (path?: string) => path?.split('/').pop()

/** Subsonic's `Child` (a song) as koel's `Song`. */
const toSong = (child: Record<string, any>): Song => ({
  type: 'songs',
  id: child.id,
  title: child.title,
  length: child.duration ?? 0,
  play_count: child.playCount ?? 0,
  rating: child.userRating ?? 0,
  favorite: Boolean(child.starred),
  created_at: child.created ?? '',
  owner_id: '1',
  album_id: child.albumId ?? '',
  album_name: child.album ?? '',
  album_cover: coverUrl(child.coverArt),
  artist_id: child.artistId ?? '',
  artist_name: child.artist ?? '',
  album_artist_id: child.albumArtists?.[0]?.id ?? child.artistId ?? '',
  album_artist_name: child.displayAlbumArtist ?? child.artist ?? '',
  genre: child.genre ?? '',
  track: child.track ?? null,
  disc: child.discNumber ?? 1,
  year: child.year ?? null,
  lyrics: '',
  is_public: true,
  is_external: false,
  mbid: child.musicBrainzId ?? null,
  file_size: child.size ?? null,
  basename: basename(child.path),
})

/** Subsonic's `AlbumID3` as koel's `Album`. */
const toAlbum = (album: Record<string, any>): Album => ({
  type: 'albums',
  id: album.id,
  artist_id: album.artistId ?? '',
  artist_name: album.artist ?? '',
  name: album.name,
  cover: coverUrl(album.coverArt),
  thumbnail: coverUrl(album.coverArt, 300),
  created_at: album.created ?? '',
  mbid: album.musicBrainzId ?? null,
  year: album.year ?? null,
  length: album.duration ?? 0,
  is_external: false,
  favorite: Boolean(album.starred),
  rating: album.userRating ?? 0,
  permissions: { edit: true },
})

/** Subsonic's `ArtistID3` as koel's `Artist`. */
const toArtist = (artist: Record<string, any>): Artist => ({
  type: 'artists',
  id: artist.id,
  name: artist.name,
  image: coverUrl(artist.coverArt),
  created_at: '',
  mbid: artist.musicBrainzId ?? null,
  is_external: false,
  favorite: Boolean(artist.starred),
  rating: artist.userRating ?? 0,
  permissions: { edit: true },
})

/**
 * Subsonic's `Playlist` as koel's, with what píxiū's API adds (`folderId`,
 * `rules`). Subsonic calls mirrors of watched playlists and smart playlists
 * read-only; the player may still edit and delete smart playlists.
 */
const toPlaylist = (playlist: Record<string, any>): Playlist => {
  const smart = Array.isArray(playlist.rules)
  const mirror = Boolean(playlist.readonly) && !smart

  return {
    type: 'playlists',
    id: playlist.id,
    owner_id: '1',
    name: playlist.name,
    description: playlist.comment ?? '',
    folder_id: playlist.folderId ?? null,
    is_smart: smart,
    is_collaborative: false,
    rules: smart ? playlist.rules : [],
    cover: playlist.coverArt ? coverUrl(playlist.coverArt) : null,
    permissions: { edit: !mirror, delete: !mirror },
  }
}

const songsOf = (list: Record<string, any> | undefined, key = 'song') =>
  ((list?.[key] ?? []) as Record<string, any>[]).map(toSong)

export const subsonic = {
  url,
  call,
  coverUrl,
  toSong,
  toAlbum,
  toArtist,
  toPlaylist,

  /** Where `<audio>` fetches a song; transcoded to `bitrate` kbps when given. */
  streamUrl: (id: string, bitrate?: number) =>
    bitrate ? url('stream', { id, format: 'mp3', maxBitRate: bitrate }) : url('stream', { id }),

  /** An album list (`random`, `newest`, `frequent`, `recent`, …). */
  async albumList(type: string, size: number, offset = 0) {
    const body = await call('getAlbumList2', { type, size, offset })
    return ((body.albumList2?.album ?? []) as Record<string, any>[]).map(toAlbum)
  },

  /** Every album artist, alphabetically. */
  async artists() {
    const body = await call('getArtists')
    return ((body.artists?.index ?? []) as Record<string, any>[]).flatMap(index =>
      ((index.artist ?? []) as Record<string, any>[]).map(toArtist),
    )
  },

  /** An album's songs, in disc and track order. */
  async albumSongs(id: string) {
    const body = await call('getAlbum', { id })
    return ((body.album?.song ?? []) as Record<string, any>[]).map(toSong)
  },

  async randomSongs(size: number, genre?: string) {
    const body = await call('getRandomSongs', { size, genre })
    return songsOf(body.randomSongs)
  },

  async song(id: string) {
    return toSong((await call('getSong', { id })).song)
  },

  async album(id: string) {
    return toAlbum((await call('getAlbum', { id })).album)
  },

  async artist(id: string) {
    return toArtist((await call('getArtist', { id })).artist)
  },

  /** An artist's albums, newest first as Subsonic lists them. */
  async artistAlbums(id: string) {
    const body = await call('getArtist', { id })
    return ((body.artist?.album ?? []) as Record<string, any>[]).map(toAlbum)
  },

  async artistInfo(id: string) {
    return (await call('getArtistInfo2', { id })).artistInfo2 ?? {}
  },

  async starredSongs() {
    return songsOf((await call('getStarred2')).starred2)
  },

  star: (ids: string[]) => call('star', { id: ids }, true),
  unstar: (ids: string[]) => call('unstar', { id: ids }, true),
  setRating: (id: string, rating: number) => call('setRating', { id, rating }),

  /** Counts a play (`submission`), or says what is playing now. */
  scrobble: (id: string, submission: boolean, time?: number) => call('scrobble', { id, submission, time }),

  async search(query: string, count: number) {
    const body = await call('search3', { query, songCount: count, albumCount: count, artistCount: count })
    const result = body.searchResult3 ?? {}

    return {
      songs: songsOf(result),
      albums: ((result.album ?? []) as Record<string, any>[]).map(toAlbum),
      artists: ((result.artist ?? []) as Record<string, any>[]).map(toArtist),
    }
  },

  async playlists() {
    const body = await call('getPlaylists')
    return ((body.playlists?.playlist ?? []) as Record<string, any>[]).map(toPlaylist)
  },

  async playlistSongs(id: string) {
    return songsOf((await call('getPlaylist', { id })).playlist, 'entry')
  },

  async createPlaylist(name: string, songIds: string[]) {
    return toPlaylist((await call('createPlaylist', { name, songId: songIds }, true)).playlist)
  },

  /** Replaces a playlist's songs. */
  setPlaylistSongs: (id: string, songIds: string[]) =>
    call('createPlaylist', { playlistId: id, songId: songIds }, true),

  updatePlaylist: (id: string, data: { name?: string; comment?: string; songIdToAdd?: string[] }) =>
    call('updatePlaylist', { playlistId: id, ...data }, true),

  deletePlaylist: (id: string) => call('deletePlaylist', { id }),

  /**
   * A song's lyrics as koel reads them: LRC when píxiū has synced lines,
   * plain text otherwise, empty when it has none.
   */
  async lyrics(id: string) {
    const list = ((await call('getLyricsBySongId', { id })).lyricsList?.structuredLyrics ?? []) as Record<string, any>[]
    const chosen = list.find(lyrics => lyrics.synced) ?? list[0]

    if (!chosen) {
      return ''
    }

    const lines = (chosen.line ?? []) as Array<{ start?: number; value: string }>

    if (!chosen.synced) {
      return lines.map(line => line.value).join('\n')
    }

    const stamp = (ms: number) => {
      const minutes = String(Math.floor(ms / 60000)).padStart(2, '0')
      const seconds = String(Math.floor((ms % 60000) / 1000)).padStart(2, '0')
      const hundredths = String(Math.floor((ms % 1000) / 10)).padStart(2, '0')

      return `[${minutes}:${seconds}.${hundredths}]`
    }

    return lines.map(line => `${stamp(line.start ?? 0)}${line.value}`).join('\n')
  },

  /** The saved queue, as koel's `QueueState`. */
  async playQueue(): Promise<QueueState> {
    const queue = (await call('getPlayQueue')).playQueue ?? {}
    const songs = songsOf(queue, 'entry')

    return {
      type: 'queue-states',
      songs,
      current_song: songs.find(song => song.id === queue.current) ?? null,
      playback_position: Math.floor((queue.position ?? 0) / 1000),
    }
  },

  savePlayQueue: (ids: string[], current: string | null, positionSeconds: number) =>
    call('savePlayQueue', { id: ids, current, position: Math.floor(positionSeconds * 1000) }, true),
}
