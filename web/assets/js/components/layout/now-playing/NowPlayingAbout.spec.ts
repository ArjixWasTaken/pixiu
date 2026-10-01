import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { useAlbumStore } from '@/stores/albumStore'
import { useArtistStore } from '@/stores/artistStore'
import { encyclopediaService } from '@/services/encyclopediaService'
import { useNowPlaying } from '@/composables/useNowPlaying'
import Component from './NowPlayingAbout.vue'

describe('nowPlayingAbout.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      useNowPlaying().about.value = 'Artist'
    },
  })

  it('links the artist, and says when nothing is known about them', async () => {
    const artist = h.factory('artist').make({ id: 'ar-5', name: 'Somebody' })
    h.mock(useArtistStore(), 'resolve').mockResolvedValue(artist)
    h.mock(encyclopediaService, 'fetchForArtist').mockResolvedValue(null)

    h.render(Component, { props: { song: h.factory('song').make({ artist_id: 'ar-5' }) } })

    await screen.findByText('píxiū knows nothing about Somebody yet.')
    expect(screen.getByRole('link', { name: 'Somebody' }).getAttribute('href')).toMatch(/artists\/ar-5$/)
    expect(screen.queryByRole('link', { name: 'Source' })).toBeNull()
  })

  it('links the album on the Album tab', async () => {
    const album = h.factory('album').make({ id: 'al-9', name: 'Some album' })
    h.mock(useAlbumStore(), 'resolve').mockResolvedValue(album)
    h.mock(encyclopediaService, 'fetchForAlbum').mockResolvedValue(null)
    h.mock(useArtistStore(), 'resolve').mockResolvedValue(undefined)
    useNowPlaying().about.value = 'Album'

    h.render(Component, { props: { song: h.factory('song').make({ album_id: 'al-9' }) } })

    const link = await screen.findByRole('link', { name: 'Some album' })
    expect(link.getAttribute('href')).toMatch(/albums\/al-9$/)
  })
})
