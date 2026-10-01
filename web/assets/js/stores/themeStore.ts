import { reactive } from 'vue'
import { preferenceStore as preferences } from '@/stores/preferenceStore'
import themes from '@/config/themes'

const DARK_QUERY = '(prefers-color-scheme: dark)'

const systemPrefersDark = () => typeof window.matchMedia === 'function' && window.matchMedia(DARK_QUERY).matches

export const themeStore = {
  state: reactive({
    themes,
    /** Whether the dark variant shows, as `applyMode` last set it. */
    dark: true,
  }),

  init() {
    this.setTheme(this.getCurrentTheme())

    // Following the system: follow it when it changes, too.
    window.matchMedia?.(DARK_QUERY).addEventListener?.('change', () => {
      if (preferences.dark_mode === null) {
        this.applyMode()
      }
    })
  },

  get all() {
    return this.state.themes
  },

  setTheme(theme?: Theme | Theme['id']) {
    if (theme === undefined) {
      this.setTheme(this.getCurrentTheme())
      return
    }

    if (typeof theme === 'string') {
      theme = this.getThemeById(theme) ?? this.getDefaultTheme()
    }

    preferences.theme = theme.id
    this.applyMode()
  },

  get darkMode() {
    const chosen = preferences.dark_mode
    if (chosen === null) {
      return systemPrefersDark()
    }
    return chosen ?? true
  },

  /** Dark, light, or `null` to follow the system. */
  setDarkMode(dark: boolean | null) {
    preferences.dark_mode = dark
    this.applyMode()
  },

  /**
   * Selects the scheme's tokens: `data-mode="<scheme>-dt|-lt"` on the root,
   * except for Baseline, whose tokens hang off `data-theme="dark|light"`.
   * The cover's scheme stands on Orange's.
   */
  applyMode() {
    const root = document.documentElement
    const current = this.getCurrentTheme().id
    const scheme = current === 'cover' ? 'orange' : current
    this.state.dark = this.darkMode

    if (scheme === 'baseline') {
      root.removeAttribute('data-mode')
      root.setAttribute('data-theme', this.darkMode ? 'dark' : 'light')
    } else {
      root.removeAttribute('data-theme')
      root.setAttribute('data-mode', `${scheme}-${this.darkMode ? 'dt' : 'lt'}`)
    }
  },

  isCurrentTheme(theme: Theme | Theme['id']) {
    const currentTheme = this.getCurrentTheme()
    return typeof theme === 'string' ? currentTheme.id === theme : currentTheme.id === theme.id
  },

  getThemeById(id: Theme['id']) {
    return this.state.themes.find(theme => theme.id === id)
  },

  getDefaultTheme() {
    return this.getThemeById('cover')!
  },

  /** Whether the colors come from the cover playing. */
  get followsCover() {
    return this.getCurrentTheme().id === 'cover'
  },

  getCurrentTheme() {
    return preferences.theme ? (this.getThemeById(preferences.theme) ?? this.getDefaultTheme()) : this.getDefaultTheme()
  },

  isValidTheme(id: Theme['id']) {
    return this.getThemeById(id) !== undefined
  },
}
