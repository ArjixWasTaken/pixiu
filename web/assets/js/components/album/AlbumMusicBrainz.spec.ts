import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { huntingService } from '@/services/huntingService'
import type { AlbumDetails } from '@/services/huntingService'
import Component from './AlbumMusicBrainz.vue'

const details = (changes: Partial<AlbumDetails>): AlbumDetails => ({
  title: 'Groovy',
  artist: 'Kevin MacLeod',
  year: 2016,
  enrichment: 'matched',
  enriched_at: null,
  mbid: 'f00',
  candidates: [],
  source: 'youtube_music',
  youtube_url: null,
  tracks: [],
  ...changes,
})

describe('albumMusicBrainz.vue', () => {
  const h = createHarness()

  it('links the matched release, and says when it was matched', async () => {
    const enrichedAt = new Date(Date.now() - 3 * 86_400_000).toISOString()
    h.mock(huntingService, 'albumDetails').mockResolvedValue(details({ enriched_at: enrichedAt }))
    h.render(Component, { props: { album: h.factory('album').make() } })

    const link = await screen.findByRole('link', { name: 'a release' })
    expect(link.getAttribute('href')).toBe('https://musicbrainz.org/release/f00')
    expect(link.parentElement!.textContent!.replace(/\s+/g, ' ').trim()).toBe(
      'Matched to a release 3 days ago; its tags follow it.',
    )
  })
})
