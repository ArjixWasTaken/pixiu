import { screen } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { commonStore } from '@/stores/commonStore'
import { encyclopediaService } from '@/services/encyclopediaService'
import Component from './AlbumInfo.vue'

describe('albumInfo.vue', () => {
  const h = createHarness()

  const renderComponent = async (mode: EncyclopediaDisplayMode = 'aside', info?: AlbumInfo) => {
    commonStore.state.uses_musicbrainz = true

    if (info === undefined) {
      info = h.factory('album-info').make()
    }

    const album = h.factory('album').make({ name: 'IV' })
    const fetchMock = h.mock(encyclopediaService, 'fetchForAlbum').mockResolvedValue(info)

    const rendered = h.render(Component, {
      props: {
        album,
        mode,
      },
      global: {
        stubs: {
          TrackList: h.stub(),
          AlbumThumbnail: h.stub('thumbnail'),
        },
      },
    })

    await h.tick(1)
    expect(fetchMock).toHaveBeenCalledWith(album)

    return {
      ...rendered,
      album,
    }
  }

  it.each<[EncyclopediaDisplayMode]>([['aside'], ['full']])('renders in %s mode', async mode => {
    await renderComponent(mode)

    screen.getByTestId('album-info-tracks')

    if (mode === 'aside') {
      screen.getByTestId('thumbnail')
    } else {
      expect(screen.queryByTestId('thumbnail')).toBeNull()
    }

    expect(screen.getByTestId('album-info').classList.contains(mode)).toBe(true)
  })

  it('says when Wikipedia has nothing, without a source', async () => {
    commonStore.state.uses_musicbrainz = true
    h.mock(encyclopediaService, 'fetchForAlbum').mockResolvedValue(null)
    h.render(Component, {
      props: { album: h.factory('album').make({ name: 'IV' }), mode: 'full' },
      global: { stubs: { TrackList: h.stub(), AlbumThumbnail: h.stub('thumbnail') } },
    })

    await screen.findByText('píxiū found nothing about IV on Wikipedia.')
    expect(screen.queryByRole('link', { name: 'Source' })).toBeNull()
  })
})
