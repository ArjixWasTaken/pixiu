/**
 * píxiū's hunting API: searching a platform and grabbing, watches, jobs,
 * orphans, offerings, the YouTube Music account, and settings.
 */
import { http } from '@/services/http'
import { subsonic } from '@/services/subsonic'

export type Standing = 'hoarded' | 'pending' | 'missing'

export interface HuntTrack {
  id: string
  title: string
  artist: string
  album: string | null
  length: number | null
  cover: string | null
  is_video: boolean
  standing: Standing
  /** The album holding the library's copy. */
  library_album?: string | null
}

export interface HuntAlbum {
  id: string
  title: string
  artist: string
  year: number | null
  kind: string
  cover: string | null
  standing: Standing
  /** The library's copy. */
  library_album?: string | null
}

export type WatchKind = 'playlist' | 'liked_music' | 'artist'

export interface Watch {
  id: number
  kind: WatchKind
  name: string
  image: string | null
  /** The platform it follows something on, like `deezer`. */
  platform: string
  link: string
  include_singles: boolean
  only_new: boolean
  releases_known: number
  created_at: string
  last_synced_at: string | null
  next_sync_at: string
  status: {
    state: 'synced' | 'never_synced' | 'syncing' | 'queued' | 'waiting' | 'failed'
    error: string | null
  }
  jobs: { queued: number; failed: number }
  songs: { have: number; total: number } | null
  playlist_id: string | null
}

export interface ExcludedSong {
  /** The song on its platform, like `youtube_music:dQw4w9WgXcQ`. */
  key: string
  title: string | null
  artist: string | null
  excluded_at: string
}

export interface PlaylistWatch {
  watch: {
    id: number
    kind: WatchKind
    name: string
    platform: string
    link: string
    last_synced_at: string | null
  }
  coming: Array<{
    key: string
    title: string | null
    artist: string | null
    /** Its download, when there is one. */
    job: { state: 'queued' | 'running' | 'paused' | 'failed' | 'done'; error: string | null } | null
  }>
  excluded: ExcludedSong[]
}

export type JobState = 'queued' | 'running' | 'done' | 'failed' | 'paused'

export interface HuntJob {
  id: number
  kind: 'download' | 'album' | 'watch_sync' | 'lookup'
  state: JobState
  title: string
  error: string | null
  progress: number | null
  family: { total: number; done: number; failed: number; running: number; waiting: number } | null
  created_at: string
  finished_at: string | null
}

export interface Orphan {
  song: Song
  reason: string
  released_at: string | null
  size: number
}

export interface OfferingFile {
  id: number
  file_name: string
  archive: string | null
  size: number
  readable: boolean
  error: string | null
  title: string
  artist: string
  album: string
  album_artist: string | null
  track: number | null
  disc: number | null
  year: number | null
  genre: string | null
  length: number
}

export interface OfferingBatch {
  batch: string
  files: OfferingFile[]
}

export type SessionStateName = 'none' | 'valid' | 'degraded' | 'expired'

export interface SessionHealth {
  state: SessionStateName
  connected_at: string | null
  last_verified: string | null
  last_refreshed: string | null
  expired_at: string | null
  last_error: string | null
}

export interface Sources {
  health: SessionHealth
  events: Array<{ message: string; problem: boolean; created_at: string }>
  login_open: boolean
}

export interface Settings {
  albums_not_looked_up: number
}

/** The platform something was downloaded from, and its page there. */
export interface SourceLink {
  platform: string
  /** The platform's name, like "YouTube Music". */
  name: string
  url: string | null
}

export interface SongInfo {
  format: string
  size: number
  origin: 'offering' | 'download'
  source_name: string | null
  source_archive: string | null
  /** Where it was downloaded from; `null` for uploads. */
  source: SourceLink | null
  mbid: string | null
  isrc: string | null
  added_at: string
  lyrics: string
  kept: Array<{ why: string; excludable_from: number | null }>
}

export interface ReleaseCandidate {
  id: string
  title: string
  artist: string
  date: string | null
  country: string | null
  format: string | null
  track_count: number
  score: number
}

export interface AlbumDetails {
  title: string
  artist: string
  year: number | null
  enrichment: 'matched' | 'review' | 'unmatched' | null
  enriched_at: string | null
  mbid: string | null
  candidates: ReleaseCandidate[]
  /** Where it was downloaded from; `null` for uploads. */
  source: SourceLink | null
  tracks: Array<{ id: string; title: string; track: number | null; disc: number | null }>
}

export interface HuntingSummary {
  session: SessionStateName
  orphans: number
  offerings: number
  jobs: { running: number; waiting: number; failed: number }
  /** For admins: how many registrations wait for them. */
  registrations: number
}

export const huntingService = {
  summary: () => http.silently.get<HuntingSummary>('hunting'),

  search: (q: string, platform: string) =>
    http.get<{ tracks: HuntTrack[]; albums: HuntAlbum[] }>(
      `hunt?q=${encodeURIComponent(q)}&platform=${encodeURIComponent(platform)}`,
    ),
  grabTrack: (track: HuntTrack) =>
    http.post('hunt/tracks', { id: track.id, title: `${track.artist} — ${track.title}` }),
  grabAlbum: (album: HuntAlbum) =>
    http.post('hunt/albums', { id: album.id, title: `${album.artist} — ${album.title}` }),

  watches: () => http.get<Watch[]>('watches'),
  addWatch: (target: string, onlyNew = false, singles = false) =>
    http.post('watches', { target, only_new: onlyNew, singles }),
  removeWatch: (id: number) => http.delete(`watches/${id}`),
  syncWatch: (id: number) => http.post(`watches/${id}/sync`),
  exclude: (watchId: number, song: string) => http.post(`watches/${watchId}/exclusions`, { song }),
  include: (watchId: number, key: string) => http.delete(`watches/${watchId}/exclusions/${encodeURIComponent(key)}`),
  playlistWatch: (playlistId: string) => http.get<PlaylistWatch | null>(`playlists/${playlistId}/watch`),

  jobs: () => http.silently.get<HuntJob[]>('jobs'),
  retryJob: (id: number) => http.post(`jobs/${id}/retry`),
  clearFinishedJobs: () => http.delete('jobs/finished'),

  async orphans() {
    const result = await http.get<{
      orphans: Array<Omit<Orphan, 'song'> & { song: Record<string, any> }>
      total_size: number
    }>('orphans')

    return {
      orphans: result.orphans.map(orphan => ({ ...orphan, song: subsonic.toSong(orphan.song) })) as Orphan[],
      totalSize: result.total_size,
    }
  },
  keepOrphans: (songs: Song[]) => http.post<{ kept: number }>('orphans/keep', { songs: songs.map(({ id }) => id) }),
  deleteOrphans: (songs: Song[] | 'all') =>
    http.post<{ deleted: number }>(
      'orphans/delete',
      songs === 'all' ? { all: true } : { songs: songs.map(({ id }) => id) },
    ),

  offerings: () => http.get<OfferingBatch[]>('offerings'),
  acceptBatch: (batch: string) =>
    http.post<{ albums: number; failures: Array<{ file_name: string; error: string }> }>(
      `offerings/batches/${batch}/accept`,
    ),
  discardBatch: (batch: string) => http.delete(`offerings/batches/${batch}`),
  discardOffering: (id: number) => http.delete(`offerings/${id}`),

  sources: () => http.get<Sources>('sources'),
  validateSession: () => http.post<SessionHealth>('sources/validate'),
  refreshSession: () => http.post<SessionHealth>('sources/refresh'),
  disconnectSession: () => http.post('sources/disconnect'),
  openLogin: () => http.post('sources/login'),
  cancelLogin: () => http.delete('sources/login'),
  loginStatus: () =>
    http.silently.get<{ open: boolean; logged_in: boolean; host: string | null }>('sources/login/status'),
  finishLogin: () => http.post<SessionHealth>('sources/login/finish'),

  settings: () => http.get<Settings>('settings'),
  lookUpAll: () => http.post<{ queued: number }>('settings/lookup-all'),

  songInfo: (song: Song) => http.get<SongInfo>(`songs/${song.id}/info`),
  albumDetails: (album: Album) => http.get<AlbumDetails>(`albums/${album.id}/details`),
  editAlbum: (
    album: Album,
    data: {
      title: string
      artist: string
      year: number | null
      tracks: Array<{ id: string; title: string; track: number | null }>
    },
  ) => http.put(`albums/${album.id}`, data),
  lookUpAlbum: (album: Album, release?: string) => http.post(`albums/${album.id}/lookup`, release ? { release } : {}),
}
