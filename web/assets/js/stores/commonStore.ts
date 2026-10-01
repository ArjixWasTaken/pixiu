import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { useViewport } from '@/composables/useViewport'
import { http } from '@/services/http'
import { subsonic } from '@/services/subsonic'
import { logger } from '@/utils/logger'
import { usePlaylistFolderStore } from '@/stores/playlistFolderStore'
import { usePlaylistStore } from '@/stores/playlistStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useQueueStore } from '@/stores/queueStore'
import { useThemeStore } from '@/stores/themeStore'
import { useUserStore } from '@/stores/userStore'
import { useHuntingStore } from '@/stores/huntingStore'
import type { HuntingSummary } from '@/services/huntingService'

const initialState = () => ({
  allows_download: false,
  assignable_roles: [] as Array<{ id: Role; label: string; description: string }>,
  cdn_url: '',
  current_user: null! as CurrentUser,
  current_version: '',
  latest_version: '',
  playlists: [] as Playlist[],
  playlist_folders: [] as PlaylistFolder[],
  uses_musicbrainz: false,
  users: [] as User[],
  storage_driver: 'local',
  supports_presigned_uploads: false,
  song_count: 0,
  song_length: 0,
  queue_state: {
    type: 'queue-states',
    songs: [],
    current_song: null,
    playback_position: 0,
  } as QueueState,
  supports_batch_downloading: false,
  supports_transcoding: false,
  dir_separator: '/',
  hunting: undefined as HuntingSummary | undefined,
})

type CommonStoreState = ReturnType<typeof initialState>

/** What the server tells the player at start-up, and the start-up itself. */
export const useCommonStore = defineStore('common', () => {
  const state = reactive<CommonStoreState>(initialState())

  const init = async () => {
    const [bootstrap, playlists, queueState] = await Promise.all([
      http.get<CommonStoreState>('bootstrap'),
      http.get<Record<string, any>[]>('playlists').then(playlists => playlists.map(subsonic.toPlaylist)),
      // A queue that fails to load is not worth failing start-up over.
      subsonic.playQueue().catch(error => {
        logger.error(error)
        return initialState().queue_state
      }),
    ])

    Object.assign(state, bootstrap, { playlists, queue_state: queueState })

    // Only enable transcoding on mobile
    state.supports_transcoding = state.supports_transcoding && useViewport().isTouch.value

    useUserStore().init(state.current_user)
    usePreferenceStore().init(state.current_user.preferences)
    usePlaylistStore().init(state.playlists)
    usePlaylistFolderStore().init(state.playlist_folders)
    useQueueStore().init(state.queue_state)
    useThemeStore().init()
    useHuntingStore().init(state.hunting)

    return state
  }

  return { state, init }
})
