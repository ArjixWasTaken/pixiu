import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useCommonStore } from '@/stores/commonStore'
import { encyclopediaService } from '@/services/encyclopediaService'
import Component from './ArtistInfo.vue'

describe('artistInfo.vue', () => {
  const h = createHarness()

  const renderComponent = async (mode: EncyclopediaDisplayMode = 'aside', info?: ArtistInfo) => {
    useCommonStore().state.uses_musicbrainz = true
    info = info ?? h.factory('artist-info').make()
    const artist = h.factory('artist').make({ name: 'Led Zeppelin' })

    const fetchMock = h.mock(encyclopediaService, 'fetchForArtist').mockResolvedValue(info)

    const rendered = h.render(Component, {
      props: {
        artist,
        mode,
      },
      global: {
        stubs: {
          ArtistThumbnail: h.stub('thumbnail'),
        },
      },
    })

    await h.tick(1)
    expect(fetchMock).toHaveBeenCalledWith(artist)

    return {
      ...rendered,
      artist,
    }
  }

  it.each<[EncyclopediaDisplayMode]>([['aside'], ['full']])('renders in %s mode', async mode => {
    await renderComponent(mode)

    if (mode === 'aside') {
      screen.getByTestId('thumbnail')
    } else {
      expect(screen.queryByTestId('thumbnail')).toBeNull()
    }

    expect(screen.getByTestId('artist-info').classList.contains(mode)).toBe(true)
  })

  it('says when Wikipedia has nothing, without a source', async () => {
    useCommonStore().state.uses_musicbrainz = true
    h.mock(encyclopediaService, 'fetchForArtist').mockResolvedValue(null)
    h.render(Component, {
      props: { artist: h.factory('artist').make({ name: 'Led Zeppelin' }), mode: 'full' },
      global: { stubs: { ArtistThumbnail: h.stub('thumbnail') } },
    })

    await screen.findByText('píxiū found nothing about Led Zeppelin on Wikipedia.')
    expect(screen.queryByRole('link', { name: 'Source' })).toBeNull()
  })
})
