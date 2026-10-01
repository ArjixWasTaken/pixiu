import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'

/** How the song stands, as useOfflinePlayback would say. */
const offline = vi.hoisted(() => ({
  swReady: true,
  cached: false,
  caching: false,
  error: undefined as string | undefined,
  makeAvailableOffline: vi.fn(),
  removeOfflineCache: vi.fn(),
}))

vi.mock('@/composables/useOfflinePlayback', () => ({
  useOfflinePlayback: () => ({
    swReady: offline.swReady,
    isCached: () => offline.cached,
    isCaching: () => offline.caching,
    getCachingProgress: () => 0.4,
    hasCachingError: () => Boolean(offline.error),
    getCachingError: () => offline.error,
    makeAvailableOffline: offline.makeAvailableOffline,
    removeOfflineCache: offline.removeOfflineCache,
  }),
}))

import Component from './OfflineButton.vue'

describe('offlineButton.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      Object.assign(offline, { swReady: true, cached: false, caching: false, error: undefined })
      offline.makeAvailableOffline.mockClear()
      offline.removeOfflineCache.mockClear()
    },
  })

  const renderButton = () => {
    const playable = h.factory('song').make()
    h.render(Component, { props: { playable } })
    return playable
  }

  it('makes a song available offline: a down arrow in a circle', async () => {
    const playable = renderButton()
    const button = screen.getByRole('button', { name: 'Make available offline' })

    expect(button.querySelector('[data-icon]')!.getAttribute('data-icon')).toBe('arrow_circle_down')

    await h.user.click(button)

    expect(offline.makeAvailableOffline).toHaveBeenCalledWith(playable)
  })

  it('shows its progress while on its way', () => {
    offline.caching = true
    renderButton()

    const button = screen.getByRole('button', { name: 'Making available offline…' })
    expect(button.hasAttribute('disabled')).toBe(true)
    expect(screen.getByRole('progressbar').getAttribute('aria-valuenow')).toBe('40')
  })

  it('shows a song kept offline with a filled check, and lets it go', async () => {
    offline.cached = true
    const playable = renderButton()
    const button = screen.getByRole('button', { name: 'Remove offline copy' })

    expect(button.querySelector('[data-icon]')!.getAttribute('data-icon')).toBe('check_circle')
    expect(button.querySelector('[data-icon]')!.getAttribute('style')).toContain("'FILL' 1")

    await h.user.click(button)

    expect(offline.removeOfflineCache).toHaveBeenCalledWith(playable)
  })

  it('says what went wrong, and tries again', async () => {
    offline.error = 'HTTP 500'
    const playable = renderButton()

    await h.user.click(screen.getByRole('button', { name: /Couldn’t make it available offline \(HTTP 500\)/ }))

    expect(offline.makeAvailableOffline).toHaveBeenCalledWith(playable)
  })

  it('shows nothing where no service worker keeps songs', () => {
    offline.swReady = false
    renderButton()

    expect(screen.queryByRole('button')).toBeNull()
  })
})
