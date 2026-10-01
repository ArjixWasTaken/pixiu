import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { useOverviewStore } from '@/stores/overviewStore'
import TopArtists from './TopArtists.vue'

describe('topArtists.vue', () => {
  const h = createHarness()

  it('displays the artists', () => {
    useOverviewStore().state.mostPlayedArtists = h.factory('artist').make(6)
    expect(
      h
        .render(TopArtists, {
          global: {
            stubs: {
              ArtistCard: h.stub('artist-card'),
            },
          },
        })
        .getAllByTestId('artist-card'),
    ).toHaveLength(6)
  })
})
