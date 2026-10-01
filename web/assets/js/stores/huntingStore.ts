/**
 * What the player knows about hunting between screens: the YouTube Music
 * session's state and the counts the sidebar shows. A server-sent event
 * stream keeps it live, and tells screens when the job board changed.
 */
import { reactive } from 'vue'
import { authService } from '@/services/authService'
import { huntingService } from '@/services/huntingService'
import type { HuntingSummary, PlaylistWatch, SessionStateName } from '@/services/huntingService'
import { cache } from '@/services/cache'
import { playlistStore } from '@/stores/playlistStore'
import { eventBus } from '@/utils/eventBus'
import { logger } from '@/utils/logger'

let source: EventSource | null = null

export const huntingStore = {
  state: reactive<HuntingSummary>({
    session: 'none',
    orphans: 0,
    offerings: 0,
    jobs: { running: 0, waiting: 0, failed: 0 },
    registrations: 0,
  }),

  /** What each playlist mirrors, by playlist id; `null` for playlists of the admin's own. */
  playlistWatches: reactive<Record<Playlist['id'], PlaylistWatch | null>>({}),

  async fetchPlaylistWatch(playlist: Playlist) {
    // The admin's own playlists mirror nothing.
    if (playlist.permissions.edit) {
      this.playlistWatches[playlist.id] = null
      return null
    }

    this.playlistWatches[playlist.id] = await huntingService.playlistWatch(playlist.id)
    return this.playlistWatches[playlist.id]
  },

  /**
   * Excludes songs from a watched playlist: they leave its mirror and
   * become orphans unless something else keeps them.
   */
  async exclude(watchId: number, songs: Song[]) {
    for (const song of songs) {
      await huntingService.exclude(watchId, song.id)
    }

    await this.exclusionsChanged()
  },

  async include(watchId: number, videoId: string) {
    await huntingService.include(watchId, videoId)
    await this.exclusionsChanged()
  },

  async exclusionsChanged() {
    playlistStore.state.playlists
      .filter(playlist => !playlist.permissions.edit)
      .forEach(playlist => cache.remove(['playlist.songs', playlist.id]))

    eventBus.emit('WATCH_EXCLUSIONS_CHANGED')
    await this.refresh()
  },

  init(summary?: HuntingSummary) {
    summary && Object.assign(this.state, summary)
    this.connect()
  },

  /** Listens to the server's events; the browser reconnects on its own. */
  connect() {
    this.disconnect()

    if (typeof EventSource === 'undefined') {
      return
    }

    const token = encodeURIComponent(authService.getApiToken() ?? '')
    source = new EventSource(`${window.KOEL.base_url}api/events?api_key=${token}`)

    source.addEventListener('jobs', () => {
      eventBus.emit('HUNT_JOBS_CHANGED')
      this.refresh()
    })

    source.addEventListener('session', event => {
      this.state.session = (event as MessageEvent<string>).data as SessionStateName
    })
  },

  disconnect() {
    source?.close()
    source = null
  },

  async refresh() {
    try {
      Object.assign(this.state, await huntingService.summary())
    } catch (error: unknown) {
      logger.error(error)
    }
  },
}
