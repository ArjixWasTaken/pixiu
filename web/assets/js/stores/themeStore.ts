import { defineStore } from 'pinia'
import { computed, reactive } from 'vue'
import { usePreferenceStore } from '@/stores/preferenceStore'
import themes from '@/config/themes'

const DARK_QUERY = '(prefers-color-scheme: dark)'

const systemPrefersDark = () => typeof window.matchMedia === 'function' && window.matchMedia(DARK_QUERY).matches

export const useThemeStore = defineStore('theme', () => {
  const preferences = usePreferenceStore()

  const state = reactive({
    themes,
    /** Whether the dark variant shows, as `applyMode` last set it. */
    dark: true,
  })

  const all = computed(() => state.themes)

  const getThemeById = (id: Theme['id']) => state.themes.find(theme => theme.id === id)

  const getDefaultTheme = () => getThemeById('cover')!

  const getCurrentTheme = () =>
    preferences.theme ? (getThemeById(preferences.theme) ?? getDefaultTheme()) : getDefaultTheme()

  /** Dark as chosen, or as the system prefers (`null`); read afresh, as the system may change. */
  const prefersDark = () => (preferences.dark_mode === null ? systemPrefersDark() : (preferences.dark_mode ?? true))

  /** Whether the colors come from the cover playing. */
  const followsCover = computed(() => getCurrentTheme().id === 'cover')

  /**
   * Selects the scheme's tokens: `data-mode="<scheme>-dt|-lt"` on the root,
   * except for Baseline, whose tokens hang off `data-theme="dark|light"`.
   * The cover's scheme stands on Orange's.
   */
  const applyMode = () => {
    const root = document.documentElement
    const current = getCurrentTheme().id
    const scheme = current === 'cover' ? 'orange' : current
    const dark = prefersDark()
    state.dark = dark

    if (scheme === 'baseline') {
      root.removeAttribute('data-mode')
      root.setAttribute('data-theme', dark ? 'dark' : 'light')
    } else {
      root.removeAttribute('data-theme')
      root.setAttribute('data-mode', `${scheme}-${dark ? 'dt' : 'lt'}`)
    }
  }

  const setTheme = (theme?: Theme | Theme['id']) => {
    if (theme === undefined) {
      setTheme(getCurrentTheme())
      return
    }

    if (typeof theme === 'string') {
      theme = getThemeById(theme) ?? getDefaultTheme()
    }

    preferences.theme = theme.id
    applyMode()
  }

  /** Dark, light, or `null` to follow the system. */
  const setDarkMode = (dark: boolean | null) => {
    preferences.dark_mode = dark
    applyMode()
  }

  const init = () => {
    setTheme(getCurrentTheme())

    // Following the system: follow it when it changes, too.
    window.matchMedia?.(DARK_QUERY).addEventListener?.('change', () => {
      if (preferences.dark_mode === null) {
        applyMode()
      }
    })
  }

  const isCurrentTheme = (theme: Theme | Theme['id']) => {
    const currentTheme = getCurrentTheme()
    return typeof theme === 'string' ? currentTheme.id === theme : currentTheme.id === theme.id
  }

  const isValidTheme = (id: Theme['id']) => getThemeById(id) !== undefined

  return {
    state,
    all,
    followsCover,
    init,
    setTheme,
    setDarkMode,
    applyMode,
    isCurrentTheme,
    getThemeById,
    getDefaultTheme,
    getCurrentTheme,
    isValidTheme,
  }
})
