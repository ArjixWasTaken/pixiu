import type { SongUpdateResult } from '@/stores/playableStore'

/** Each event and what it carries (`undefined`: nothing). */
export type Events = {
  LOG_OUT: undefined
  NEW_VERSION_DEPLOYED: undefined
  TOGGLE_SIDEBAR: undefined
  FOCUS_SEARCH_FIELD: undefined

  FULLSCREEN_TOGGLE: undefined
  PLAYBACK_STARTED: Playable
  UP_NEXT: Playable | null

  PLAYLIST_DELETED: Playlist
  PLAYLIST_CONTENT_REMOVED: { playlist: Playlist; playables: Playable[] }
  PLAYLIST_UPDATED: Playlist

  SONGS_UPDATED: SongUpdateResult
  SONGS_DELETED: Song[]
  SONG_UPLOADED: Song
  /** The job board changed on the server. */
  /** Uploaded files wait for review. */
  OFFERINGS_UPLOADED: undefined
  /** Songs were excluded from a watched playlist, or let back in. */
  WATCH_EXCLUSIONS_CHANGED: undefined
  DOWNLOAD_ARCHIVE_SAVED: undefined
}
