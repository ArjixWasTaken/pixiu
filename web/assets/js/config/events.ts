import type { SongUpdateResult } from '@/stores/playableStore'

export interface Events {
  LOG_OUT: () => void
  NEW_VERSION_DEPLOYED: () => void
  TOGGLE_SIDEBAR: () => void
  FOCUS_SEARCH_FIELD: () => void
  SEARCH_KEYWORDS_CHANGED: (keywords: string) => void

  FULLSCREEN_TOGGLE: () => void
  PLAYBACK_STARTED: (playable: Playable) => void
  UP_NEXT: (playable: Playable | null) => void

  PLAYLIST_DELETED: (playlist: Playlist) => void
  PLAYLIST_CONTENT_REMOVED: (playlist: Playlist, playables: Playable[]) => void
  PLAYLIST_UPDATED: (playlist: Playlist) => void

  SONGS_UPDATED: (result: SongUpdateResult) => void
  SONGS_DELETED: (songs: Song[]) => void
  SONG_UPLOADED: (song: Song) => void
  /** The job board changed on the server. */
  HUNT_JOBS_CHANGED: () => void
  /** Uploaded files wait for review. */
  OFFERINGS_UPLOADED: () => void
  /** Songs were excluded from a watched playlist, or let back in. */
  WATCH_EXCLUSIONS_CHANGED: () => void
  DOWNLOAD_ARCHIVE_SAVED: () => void
}
