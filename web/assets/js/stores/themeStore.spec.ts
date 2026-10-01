import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useThemeStore } from '@/stores/themeStore'
describe('themeStore', () => {
  createHarness({
    afterEach: () => {
      document.documentElement.removeAttribute('data-theme')
      document.documentElement.removeAttribute('data-mode')
      usePreferenceStore().state.theme = 'orange'
      usePreferenceStore().state.dark_mode = true
    },
  })

  it('initializes with the default scheme', () => {
    useThemeStore().init()

    // The cover's colors, on Orange's scheme.
    expect(usePreferenceStore().state.theme).toBe(useThemeStore().getDefaultTheme().id)
    expect(document.documentElement.getAttribute('data-mode')).toBe('orange-dt')
  })

  it('selects a scheme through data-mode', () => {
    useThemeStore().setTheme('red')

    expect(document.documentElement.getAttribute('data-mode')).toBe('red-dt')
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(usePreferenceStore().state.theme).toBe('red')
  })

  it('selects Baseline through data-theme', () => {
    useThemeStore().setTheme('baseline')

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-mode')).toBe(false)
  })

  it('switches to light mode', () => {
    useThemeStore().setTheme('pink')
    useThemeStore().setDarkMode(false)

    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-lt')
    expect(usePreferenceStore().state.dark_mode).toBe(false)
  })

  it('follows the system when set to System', () => {
    const system = vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList)
    useThemeStore().setTheme('pink')
    useThemeStore().setDarkMode(null)

    expect(usePreferenceStore().state.dark_mode).toBeNull()
    expect(system).toHaveBeenCalledWith('(prefers-color-scheme: dark)')
    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-lt')

    system.mockReturnValue({ matches: true } as MediaQueryList)
    useThemeStore().applyMode()
    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-dt')
  })

  it('keeps a chosen mode whatever the system prefers', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList)
    useThemeStore().setTheme('pink')
    useThemeStore().setDarkMode(true)

    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-dt')
  })

  it('falls back to the default scheme for unknown ids', () => {
    useThemeStore().setTheme('classic')

    expect(usePreferenceStore().state.theme).toBe('cover')
  })

  it('follows the cover by default', () => {
    expect(useThemeStore().getDefaultTheme().id).toEqual('cover')
  })

  it('stands the cover scheme on Orange', () => {
    useThemeStore().setTheme('cover')

    expect(document.documentElement.getAttribute('data-mode')).toBe('orange-dt')
    expect(useThemeStore().followsCover).toBe(true)
  })
})
