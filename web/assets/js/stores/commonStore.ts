import isMobile from 'ismobilejs'
import { reactive } from 'vue'
import { http } from '@/services/http'
import { subsonic } from '@/services/subsonic'
import { logger } from '@/utils/logger'
import { playlistFolderStore } from '@/stores/playlistFolderStore'
import { playlistStore } from '@/stores/playlistStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { queueStore } from '@/stores/queueStore'
import { themeStore } from '@/stores/themeStore'
import { userStore } from '@/stores/userStore'
import { huntingStore } from '@/stores/huntingStore'
import type { HuntingSummary } from '@/services/huntingService'

const initialState = {
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
}

type CommonStoreState = typeof initialState

export const commonStore = {
  state: reactive<CommonStoreState>(initialState),

  async init() {
    const [bootstrap, playlists, queueState] = await Promise.all([
      http.get<CommonStoreState>('bootstrap'),
      http.get<Record<string, any>[]>('playlists').then(playlists => playlists.map(subsonic.toPlaylist)),
      // A queue that fails to load is not worth failing start-up over.
      subsonic.playQueue().catch(error => {
        logger.error(error)
        return initialState.queue_state
      }),
    ])

    Object.assign(this.state, bootstrap, { playlists, queue_state: queueState })

    // Only enable transcoding on mobile
    this.state.supports_transcoding = this.state.supports_transcoding && isMobile.any

    userStore.init(this.state.current_user)
    preferenceStore.init(this.state.current_user.preferences)
    playlistStore.init(this.state.playlists)
    playlistFolderStore.init(this.state.playlist_folders)
    queueStore.init(this.state.queue_state)
    themeStore.init()
    huntingStore.init(this.state.hunting)

    return this.state
  },
}
