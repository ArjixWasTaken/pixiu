import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { SongInfo } from '@/services/huntingService'
import { huntingService } from '@/services/huntingService'
import Component from './SongInfo.vue'

const info = (changes: Partial<SongInfo>): SongInfo => ({
  format: 'Opus 128 kbps',
  size: 4_000_000,
  origin: 'download',
  source_name: null,
  source_archive: null,
  source: null,
  mbid: null,
  isrc: null,
  added_at: new Date().toISOString(),
  lyrics: 'missing',
  kept: [],
  ...changes,
})

describe('songInfo.vue', () => {
  const h = createHarness()

  it('links the page of the platform a song was downloaded from', async () => {
    h.mock(huntingService, 'songInfo').mockResolvedValue(
      info({
        source: { platform: 'youtube_music', name: 'YouTube Music', url: 'https://music.youtube.com/watch?v=abc' },
      }),
    )
    h.render(Component, { props: { song: h.factory('song').make() } })

    const link = await screen.findByRole('link', { name: 'YouTube Music' })
    expect(link.getAttribute('href')).toBe('https://music.youtube.com/watch?v=abc')
  })

  it('names the file an upload came from', async () => {
    h.mock(huntingService, 'songInfo').mockResolvedValue(
      info({ origin: 'offering', source_name: 'song.flac', source_archive: 'album.zip' }),
    )
    h.render(Component, { props: { song: h.factory('song').make() } })

    await screen.findByText(/Upload “song\.flac” from album\.zip/)
    expect(screen.queryByRole('link', { name: 'YouTube Music' })).toBeNull()
  })
})
