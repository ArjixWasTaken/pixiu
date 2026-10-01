import { reactive, ref } from 'vue'

export const defaultPreferences: UserPreferences = {
  volume: 7,
  show_now_playing_notification: false,
  repeat_mode: 'NO_REPEAT',
  confirm_before_closing: false,
  equalizer: {
    name: 'Default',
    preamp: 0,
    gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  },
  current_equalizer_preset: {
    name: 'Default',
    preamp: 0,
    gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  },
  equalizer_presets: [],
  albums_view_mode: 'grid',
  artists_view_mode: 'grid',
  albums_sort_field: 'name',
  artists_sort_field: 'name',
  genres_sort_field: 'name',
  albums_sort_order: 'asc',
  artists_sort_order: 'asc',
  genres_sort_order: 'asc',
  albums_favorites_only: false,
  artists_favorites_only: false,
  transcode_on_mobile: false,
  transcode_quality: 128,
  lyrics_zoom_level: 1,
  theme: 'orange',
  dark_mode: true,
  active_extra_panel_tab: null,
  detect_duplicate_uploads: true,
  continuous_playback: false,
  crossfade_duration: 0,
  home_blocks_order: [],
  home_blocks_hidden: [],
  equalizer_enabled: true,
}

const STORAGE_KEY = 'preferences'

/** The preferences saved in this browser. */
const load = (): Partial<UserPreferences> => {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
  } catch {
    return {}
  }
}

const preferenceStore = {
  isTemporary: false,
  initialized: ref(false),

  state: reactive<UserPreferences>(defaultPreferences),

  init(preferences: UserPreferences = defaultPreferences) {
    // Preferences live in the browser: píxiū keeps none on the server.
    Object.assign(this.state, preferences, load())

    for (const key of ['albums_view_mode', 'artists_view_mode'] as const) {
      if ((this.state[key] as string) === 'thumbnails') {
        this.state[key] = 'grid'
      }
    }
    if (this.state.albums_view_mode === 'list') {
      this.state.albums_view_mode = 'table'
    }
    if (this.state.artists_view_mode === 'list') {
      this.state.artists_view_mode = 'table'
    }

    this.setupProxy()

    this.initialized.value = true
  },

  /**
   * Proxy the state properties, so that each can be directly accessed using the key.
   */
  setupProxy() {
    Object.keys(this.state).forEach(key => {
      Object.defineProperty(this, key, {
        get: (): any => this.get(key),
        set: (value: any): void => this.set(key, value),
        configurable: true,
      })
    })
  },

  set(key: keyof UserPreferences, value: any) {
    if (this.state[key] === value) {
      return
    }

    this.state[key] = value

    if (!this.isTemporary) {
      this.update(key, value)
    } else {
      this.isTemporary = false
    }
  },

  get(key: keyof UserPreferences) {
    return this.state?.[key]
  },

  async update(key: keyof UserPreferences, value: any) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...load(), [key]: value }))

    if (key === 'include_public_media') {
      window.location.reload()
    }
  },

  // Calling preferenceStore.temporary.volume = 7 won't trigger saving.
  // This is useful in tests as it doesn't create stray HTTP requests.
  get temporary() {
    this.isTemporary = true
    return this as unknown as ExportedType
  },
}

type ExportedType = Omit<typeof preferenceStore, 'setupProxy' | 'isTemporary'> & UserPreferences

const exported = preferenceStore as unknown as ExportedType

export { exported as preferenceStore }
