import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { preferenceStore } from '@/stores/preferenceStore'
import { themeStore } from '@/stores/themeStore'

describe('themeStore', () => {
  const h = createHarness({
    afterEach: () => {
      document.documentElement.removeAttribute('data-theme')
      document.documentElement.removeAttribute('data-mode')
      preferenceStore.state.theme = 'orange'
      preferenceStore.state.dark_mode = true
    },
  })

  it('initializes with the default scheme', () => {
    const setThemeMock = h.mock(themeStore, 'setTheme')

    themeStore.init()

    expect(setThemeMock).toHaveBeenCalledWith(themeStore.getDefaultTheme())
  })

  it('selects a scheme through data-mode', () => {
    themeStore.setTheme('red')

    expect(document.documentElement.getAttribute('data-mode')).toBe('red-dt')
    expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
    expect(preferenceStore.state.theme).toBe('red')
  })

  it('selects Baseline through data-theme', () => {
    themeStore.setTheme('baseline')

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(document.documentElement.hasAttribute('data-mode')).toBe(false)
  })

  it('switches to light mode', () => {
    themeStore.setTheme('pink')
    themeStore.setDarkMode(false)

    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-lt')
    expect(preferenceStore.state.dark_mode).toBe(false)
  })

  it('follows the system when set to System', () => {
    const system = vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList)
    themeStore.setTheme('pink')
    themeStore.setDarkMode(null)

    expect(preferenceStore.state.dark_mode).toBeNull()
    expect(system).toHaveBeenCalledWith('(prefers-color-scheme: dark)')
    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-lt')

    system.mockReturnValue({ matches: true } as MediaQueryList)
    themeStore.applyMode()
    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-dt')
  })

  it('keeps a chosen mode whatever the system prefers', () => {
    vi.spyOn(window, 'matchMedia').mockReturnValue({ matches: false } as MediaQueryList)
    themeStore.setTheme('pink')
    themeStore.setDarkMode(true)

    expect(document.documentElement.getAttribute('data-mode')).toBe('pink-dt')
  })

  it('falls back to the default scheme for unknown ids', () => {
    themeStore.setTheme('classic')

    expect(preferenceStore.state.theme).toBe('orange')
  })

  it('gets the default theme', () => {
    expect(themeStore.getDefaultTheme().id).toEqual('orange')
  })
})
