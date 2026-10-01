import { screen } from '@testing-library/vue'
import type { Mock } from 'vite-plus/test'
import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService } from '@/services/QueuePlaybackService'
import { playableStore } from '@/stores/playableStore'
import { useContextMenu } from '@/composables/useContextMenu'
import { assertOpenContextMenu } from '@/__tests__/assertions'
import Router from '@/router'
import ArtistContextMenu from './ArtistContextMenu.vue'
import Component from './ArtistCard.vue'

vi.mock('@/composables/useContextMenu')

describe('artistCard.vue', () => {
  const h = createHarness()

  const renderComponent = () => {
    const artist = h.factory('artist').make({ id: 'led-zeppelin', name: 'Led Zeppelin', favorite: false })

    return {
      ...h.render(Component, {
        props: { artist },
        global: { stubs: { AlbumArtistThumbnail: h.stub('thumbnail') } },
      }),
      artist,
    }
  }

  it('opens the artist when clicked', async () => {
    const goMock = h.mock(Router, 'go')
    renderComponent()

    await h.user.click(screen.getByTestId('artist-album-card'))

    expect(goMock).toHaveBeenCalledWith('/#/artists/led-zeppelin')
  })

  it('shuffles on double click', async () => {
    h.createAudioPlayer()
    h.mock(Router, 'go')

    const songs = h.factory('song').make(16)
    const fetchMock = h.mock(playableStore, 'fetchSongsForArtist').mockResolvedValue(songs)
    const playMock = h.mock(playbackService, 'queueAndPlay')
    const { artist } = renderComponent()

    await h.user.dblClick(screen.getByTestId('artist-album-card'))
    await h.tick()

    expect(fetchMock).toHaveBeenCalledWith(artist)
    expect(playMock).toHaveBeenCalledWith(songs, true)
  })

  it('requests context menu', async () => {
    const { openContextMenu } = useContextMenu()
    const { artist } = renderComponent()
    await h.trigger(screen.getByTestId('artist-album-card'), 'contextMenu')

    await assertOpenContextMenu(openContextMenu as Mock, ArtistContextMenu, { artist })
  })

  it('counts the albums', () => {
    h.render(Component, {
      props: { artist: h.factory('artist').make({ name: 'Led Zeppelin', album_count: 3 }) },
      global: { stubs: { AlbumArtistThumbnail: h.stub('thumbnail') } },
    })

    screen.getByText('3 albums')
  })

  it('says “Artist” when the count is unknown', () => {
    h.render(Component, {
      props: { artist: h.factory('artist').make({ name: 'Led Zeppelin', album_count: undefined }) },
      global: { stubs: { AlbumArtistThumbnail: h.stub('thumbnail') } },
    })

    screen.getByText('Artist')
  })
})
