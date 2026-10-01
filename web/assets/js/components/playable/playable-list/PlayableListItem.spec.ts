import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { playableStore } from '@/stores/playableStore'
import { createHarness } from '@/__tests__/TestHarness'

const isCachedMock = vi.fn().mockReturnValue(false)
const isCachingMock = vi.fn().mockReturnValue(false)
const hasCachingErrorMock = vi.fn().mockReturnValue(false)
const getCachingErrorMock = vi.fn().mockReturnValue(undefined)

vi.mock('@/composables/useOfflinePlayback', () => ({
  useOfflinePlayback: () => ({
    isCached: isCachedMock,
    isCaching: isCachingMock,
    hasCachingError: hasCachingErrorMock,
    getCachingError: getCachingErrorMock,
  }),
}))

import { setViewport } from '@/composables/useViewport'
import { PlayableListContextKey } from '@/config/symbols'
import Component from './PlayableListItem.vue'

describe('playableListItem.vue', () => {
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
    },
  })

  const renderComponent = (playable?: Playable, showDisc = false, context: PlayableListContext = {}) => {
    playable = playable ?? h.factory('song').make({ favorite: false })

    const row = {
      playable,
      selected: false,
    }

    const rendered = h.render(Component, {
      props: {
        item: row,
        showDisc,
      },
      global: {
        provide: {
          [<symbol>PlayableListContextKey]: [context],
        },
      },
    })

    return {
      ...rendered,
      row,
    }
  }

  it('renders song details', () => {
    const song = h.factory('song').make({
      title: 'Test Song',
      album_name: 'Test Album',
      artist_name: 'Test Artist',
      length: 1000,
      playback_state: 'Playing',
      track: 12,
      album_cover: 'https://example.com/cover.jpg',
      favorite: true,
    })

    setViewport({ mobile: false })
    renderComponent(song)

    screen.getByText('Test Song')
    screen.getByText('Test Artist · Test Album')
    screen.getByRole('button', { name: 'Remove from favorites' })
  })

  it('says when a song was played on Recently played', () => {
    const song = h.factory('song').make({
      album_name: 'Test Album',
      artist_name: 'Test Artist',
      played_at: new Date(Date.now() - 3 * 3600 * 1000).toISOString(),
    })

    setViewport({ mobile: false })
    renderComponent(song, false, { type: 'RecentlyPlayed' })

    screen.getByText('Test Artist · Test Album · played 3 hours ago')
  })

  it('leaves the play time out elsewhere', () => {
    const song = h.factory('song').make({
      album_name: 'Test Album',
      artist_name: 'Test Artist',
      played_at: new Date().toISOString(),
    })

    setViewport({ mobile: false })
    renderComponent(song, false, { type: 'Favorites' })

    screen.getByText('Test Artist · Test Album')
  })

  it('shows the track number in place of the cover in an album', () => {
    const playable = h.factory('song').make({ track: 7, album_name: 'Groovy', favorite: false })
    renderComponent(playable, false, { type: 'Album' })

    screen.getByText('7')
    expect(screen.queryByAltText('Cover image')).toBeNull()
    expect(screen.queryByText(/Groovy/)).toBeNull()
  })

  it('shows the cover elsewhere', () => {
    renderComponent(h.factory('song').make({ track: 7, album_cover: 'http://test/cover.jpg' }))

    screen.getByAltText('Cover image')
  })

  it('emits play event on double click', async () => {
    const { emitted } = renderComponent()
    await h.user.dblClick(screen.getByTestId('song-item'))
    expect(emitted().play).toBeTruthy()
  })

  it('renders disc info when showDisc is true', async () => {
    const song = h.factory('song').make({
      disc: 2,
      title: 'Test Song',
    })

    const showDisc = true
    const { getByText } = renderComponent(song, showDisc)
    expect(getByText('Disc 2')).toBeTruthy()
  })

  it('toggles favorite state when the Favorite button is clicked', async () => {
    const toggleFavoriteMock = h.mock(playableStore, 'toggleFavorite')
    const { row } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Add to favorites' }))

    expect(toggleFavoriteMock).toHaveBeenCalledWith(row.playable)
  })

  it('shows spinner when caching offline', () => {
    isCachingMock.mockReturnValue(true)
    renderComponent()
    screen.getByTitle('Caching for offline playback')
  })

  it('shows spinner instead of offline mark when caching', () => {
    isCachingMock.mockReturnValue(true)
    isCachedMock.mockReturnValue(true)
    renderComponent()
    screen.getByTitle('Caching for offline playback')
    expect(screen.queryByTitle('Available offline')).toBeNull()
  })

  it('shows error icon when caching fails', () => {
    hasCachingErrorMock.mockReturnValue(true)
    getCachingErrorMock.mockReturnValue('Network error')
    renderComponent()
    screen.getByTitle('Error: Network error')
  })
})
