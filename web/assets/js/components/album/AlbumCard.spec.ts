import { screen } from '@testing-library/vue'
import type { Mock } from 'vite-plus/test'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService } from '@/services/QueuePlaybackService'
import { playableStore } from '@/stores/playableStore'
import { useContextMenu } from '@/composables/useContextMenu'
import { setViewport } from '@/composables/useViewport'
import { assertOpenContextMenu } from '@/__tests__/assertions'
import Router from '@/router'
import AlbumContextMenu from './AlbumContextMenu.vue'
import Component from './AlbumCard.vue'

vi.mock('@/composables/useContextMenu')

describe('albumCard', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: false }),
  })

  const createAlbum = (overrides: Partial<Album> = {}) =>
    h.factory('album').make({
      id: 'al-4',
      name: 'IV',
      artist_id: 'ar-1',
      artist_name: 'Led Zeppelin',
      cover: 'https://example.com/cover.jpg',
      favorite: false,
      year: 1971,
      ...overrides,
    })

  const renderComponent = (album = createAlbum()) => ({
    ...h.render(Component, {
      props: { album },
      global: { stubs: { AlbumArtistThumbnail: h.stub('thumbnail') } },
    }),
    album,
  })

  it('shows the artist and the year', () => {
    renderComponent()
    screen.getByText('Led Zeppelin · 1971')
  })

  it('opens the album when clicked', async () => {
    const goMock = h.mock(Router, 'go')
    renderComponent()

    await h.user.click(screen.getByTestId('artist-album-card'))

    expect(goMock).toHaveBeenCalledWith('/albums/al-4')
  })

  it('shuffles on double click', async () => {
    h.createAudioPlayer()
    h.mock(Router, 'go')

    const songs = h.factory('song').make(10)
    const fetchMock = h.mock(playableStore, 'fetchSongsForAlbum').mockResolvedValue(songs)
    const shuffleMock = h.mock(playbackService, 'queueAndPlay').mockResolvedValue(void 0)
    const { album } = renderComponent()

    await h.user.dblClick(screen.getByTestId('artist-album-card'))
    await h.tick()

    expect(fetchMock).toHaveBeenCalledWith(album)
    expect(shuffleMock).toHaveBeenCalledWith(songs, true)
  })

  it('requests context menu', async () => {
    const { openContextMenu } = useContextMenu()
    const { album } = renderComponent()
    await h.trigger(screen.getByTestId('artist-album-card'), 'contextMenu')

    await assertOpenContextMenu(openContextMenu as Mock, AlbumContextMenu, { album })
  })
})
