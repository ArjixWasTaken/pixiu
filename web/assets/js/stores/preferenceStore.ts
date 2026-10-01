import { defineStore } from 'pinia'
import type { WritableComputedRef } from 'vue'
import { computed, reactive, ref } from 'vue'

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
  theme: 'cover',
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

/**
 * The user's preferences, kept in this browser (píxiū keeps none on the
 * server). Each is also read and written by its name: `preferenceStore.volume = 5`
 * saves it.
 */
export const usePreferenceStore = defineStore('preference', () => {
  const state = reactive<UserPreferences>(structuredClone(defaultPreferences))
  const initialized = ref(false)

  const get = <K extends keyof UserPreferences>(key: K) => state[key]

  const set = <K extends keyof UserPreferences>(key: K, value: UserPreferences[K]) => {
    if (state[key] === value) {
      return
    }

    state[key] = value
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...load(), [key]: value }))
  }

  const init = (preferences: UserPreferences = defaultPreferences) => {
    Object.assign(state, preferences, load())

    for (const key of ['albums_view_mode', 'artists_view_mode'] as const) {
      if ((state[key] as string) === 'thumbnails') {
        state[key] = 'grid'
      }
    }
    if (state.albums_view_mode === 'list') {
      state.albums_view_mode = 'table'
    }
    if (state.artists_view_mode === 'list') {
      state.artists_view_mode = 'table'
    }

    initialized.value = true
  }

  const byName = Object.fromEntries(
    (Object.keys(defaultPreferences) as (keyof UserPreferences)[]).map(key => [
      key,
      computed({ get: () => get(key), set: value => set(key, value) }),
    ]),
  ) as { [K in keyof UserPreferences]-?: WritableComputedRef<UserPreferences[K]> }

  return { state, initialized, get, set, init, ...byName }
})
