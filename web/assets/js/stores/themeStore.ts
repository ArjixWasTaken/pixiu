import { reactive } from 'vue'
import { preferenceStore as preferences } from '@/stores/preferenceStore'
import themes from '@/config/themes'

export const themeStore = {
  state: reactive({
    themes,
  }),

  init(theme: Theme | Theme['id'] = 'orange') {
    if (typeof theme === 'object' && theme.is_custom) {
      // custom theme from server. Add it to the list of themes.
      this.state.themes.push(theme)
    }

    this.setTheme(theme)
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
    return preferences.dark_mode ?? true
  },

  setDarkMode(dark: boolean) {
    preferences.dark_mode = dark
    this.applyMode()
  },

  /**
   * Selects the scheme's tokens: `data-mode="<scheme>-dt|-lt"` on the root,
   * except for Baseline, whose tokens hang off `data-theme="dark|light"`.
   */
  applyMode() {
    const root = document.documentElement
    const scheme = this.getCurrentTheme().id

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
    return this.getThemeById('orange')!
  },

  getCurrentTheme() {
    return preferences.theme ? (this.getThemeById(preferences.theme) ?? this.getDefaultTheme()) : this.getDefaultTheme()
  },

  isValidTheme(id: Theme['id']) {
    return this.getThemeById(id) !== undefined
  },
}
