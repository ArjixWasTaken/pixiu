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

const call = async <T = Record<string, any>>(method: string, params: Params = {}): Promise<T> => {
  const response = await fetch(url(method, params))
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

export const subsonic = {
  url,
  call,
  coverUrl,
  toSong,
  toAlbum,
  toArtist,

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

  async randomSongs(size: number) {
    const body = await call('getRandomSongs', { size })
    return ((body.randomSongs?.song ?? []) as Record<string, any>[]).map(toSong)
  },
}
