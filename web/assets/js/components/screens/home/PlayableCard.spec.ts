import { screen } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
const isCachedMock = vi.fn().mockReturnValue(false)
const isCachingMock = vi.fn().mockReturnValue(false)
const hasCachingErrorMock = vi.fn().mockReturnValue(false)
const getCachingErrorMock = vi.fn().mockReturnValue(undefined)
const makeAvailableOfflineMock = vi.fn()
const playMock = vi.fn()

vi.mock('@/services/playbackManager', () => ({
  playback: () => ({ play: playMock, pause: vi.fn(), resume: vi.fn() }),
}))

vi.mock('@/composables/useOfflinePlayback', () => ({
  useOfflinePlayback: () => ({
    isCached: isCachedMock,
    isCaching: isCachingMock,
    hasCachingError: hasCachingErrorMock,
    getCachingError: getCachingErrorMock,
    getCachingProgress: () => 0,
    makeAvailableOffline: makeAvailableOfflineMock,
    removeOfflineCache: vi.fn(),
    // A service worker runs.
    swReady: true,
  }),
}))

import Component from './PlayableCard.vue'

describe('playableCard.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      isCachedMock.mockClear()
      isCachedMock.mockReturnValue(false)
      isCachingMock.mockClear()
      isCachingMock.mockReturnValue(false)
      hasCachingErrorMock.mockClear()
      hasCachingErrorMock.mockReturnValue(false)
      getCachingErrorMock.mockClear()
      getCachingErrorMock.mockReturnValue(undefined)
      makeAvailableOfflineMock.mockClear()
      playMock.mockClear()
    },
  })

  const renderCard = (overrides: Partial<Song> = {}) => {
    const song = h.factory('song').make({
      title: 'Test Song',
      artist_name: 'Test Artist',
      length: 195,
      playback_state: 'Stopped',
      ...overrides,
    })

    return {
      ...h.render(Component, { props: { playable: song } }),
      props: { playable: song },
    }
  }

  it('renders song info', () => {
    renderCard()
    screen.getByText('Test Song')
    screen.getByText('Test Artist')
    screen.getByText('03:15')
  })

  it('highlights the playing song', () => {
    renderCard({ playback_state: 'Playing' })
    expect(screen.getByTestId('song-card').classList).toContain('playing')
  })

  it('does not highlight a stopped song', () => {
    renderCard({ playback_state: 'Stopped' })
    expect(screen.getByTestId('song-card').classList).not.toContain('playing')
  })

  it('is draggable', () => {
    renderCard()
    expect(screen.getByTestId('song-card').getAttribute('draggable')).toBe('true')
  })

  it('makes the song available offline from its button', async () => {
    const { props } = renderCard()

    await h.user.click(screen.getByRole('button', { name: 'Make available offline' }))

    expect(makeAvailableOfflineMock).toHaveBeenCalledWith(props.playable)
    expect(playMock).not.toHaveBeenCalled()
  })

  it('does so without starting playback when the button is activated with Enter', async () => {
    const { props } = renderCard()

    screen.getByRole('button', { name: 'Make available offline' }).focus()
    await h.user.keyboard('{Enter}')

    expect(makeAvailableOfflineMock).toHaveBeenCalledWith(props.playable)
    expect(playMock).not.toHaveBeenCalled()
  })

  it('plays the song when the card itself is activated with Enter', async () => {
    const { props } = renderCard()

    screen.getByTestId('song-card').focus()
    await h.user.keyboard('{Enter}')

    expect(playMock).toHaveBeenCalledWith(props.playable)
  })
})
