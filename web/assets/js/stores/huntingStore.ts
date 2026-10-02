/**
 * What the player knows about hunting between screens: the YouTube Music
 * session's state and the counts the sidebar shows. A server-sent event
 * stream keeps it live, and tells screens when the job board changed.
 */
import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { authService } from '@/services/authService'
import { huntingService } from '@/services/huntingService'
import type { HuntingSummary, PlaylistWatch, SessionStateName } from '@/services/huntingService'
import { queryClient } from '@/services/queryClient'
import { usePlaylistStore } from '@/stores/playlistStore'
import { eventBus } from '@/utils/eventBus'
import { logger } from '@/utils/logger'

export const useHuntingStore = defineStore('hunting', () => {
  let source: EventSource | null = null

  const state = reactive<HuntingSummary>({
    session: 'none',
    orphans: 0,
    offerings: 0,
    jobs: { running: 0, waiting: 0, failed: 0 },
    registrations: 0,
  })

  /** What each playlist mirrors, by playlist id; `null` for playlists of the admin's own. */
  const playlistWatches = reactive<Record<Playlist['id'], PlaylistWatch | null>>({})

  const fetchPlaylistWatch = async (playlist: Playlist) => {
    // The admin's own playlists mirror nothing.
    if (playlist.permissions.edit) {
      playlistWatches[playlist.id] = null
      return null
    }

    playlistWatches[playlist.id] = await huntingService.playlistWatch(playlist.id)
    return playlistWatches[playlist.id]
  }

  const refresh = async () => {
    try {
      Object.assign(state, await huntingService.summary())
    } catch (error: unknown) {
      logger.error(error)
    }
  }

  const exclusionsChanged = async () => {
    // The mirrors of watched playlists change with their exclusions.
    await Promise.all(
      usePlaylistStore()
        .state.playlists.filter(playlist => !playlist.permissions.edit)
        .map(playlist => queryClient.invalidateQueries({ queryKey: ['playlist', playlist.id, 'songs'] })),
    )

    eventBus.emit('WATCH_EXCLUSIONS_CHANGED')
    await refresh()
  }

  /**
   * Excludes songs from a watched playlist: they leave its mirror and
   * become orphans unless something else keeps them.
   */
  const exclude = async (watchId: number, songs: Song[]) => {
    for (const song of songs) {
      await huntingService.exclude(watchId, song.id)
    }

    await exclusionsChanged()
  }

  const include = async (watchId: number, key: string) => {
    await huntingService.include(watchId, key)
    await exclusionsChanged()
  }

  const disconnect = () => {
    source?.close()
    source = null
  }

  /** Listens to the server's events; the browser reconnects on its own. */
  const connect = () => {
    disconnect()

    if (typeof EventSource === 'undefined') {
      return
    }

    const token = encodeURIComponent(authService.getApiToken() ?? '')
    source = new EventSource(`${window.KOEL.base_url}api/events?api_key=${token}`)

    // The job board changed: what the hunting screens show is stale.
    source.addEventListener('jobs', () => {
      queryClient.invalidateQueries({ queryKey: ['hunting'] })
      refresh()
    })

    source.addEventListener('session', event => {
      state.session = (event as MessageEvent<string>).data as SessionStateName
    })
  }

  const init = (summary?: HuntingSummary) => {
    summary && Object.assign(state, summary)
    connect()
  }

  return {
    state,
    playlistWatches,
    fetchPlaylistWatch,
    exclude,
    include,
    exclusionsChanged,
    init,
    connect,
    disconnect,
    refresh,
  }
})
