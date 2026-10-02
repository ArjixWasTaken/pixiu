import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { Watch } from '@/services/huntingService'
import Component from './WatchRow.vue'

const watch: Watch = {
  id: 1,
  kind: 'artist',
  name: 'Main Artist',
  image: null,
  platform: 'deezer',
  link: 'https://www.deezer.com/artist/27',
  include_singles: false,
  only_new: true,
  releases_known: 3,
  created_at: '2026-10-01T10:00:00Z',
  last_synced_at: null,
  next_sync_at: '2026-10-02T10:00:00Z',
  status: { state: 'never_synced', error: null },
  jobs: { queued: 0, failed: 0 },
  songs: null,
  playlist_id: null,
}

describe('watchRow.vue', () => {
  const h = createHarness()

  it('opens what it follows on its platform, and shows its mark', () => {
    const { container } = h.render(Component, { props: { watch } })

    const open = screen.getByRole('button', { name: 'Open on Deezer' })
    expect(open.closest('a')!.getAttribute('href')).toBe('https://www.deezer.com/artist/27')
    expect(container.querySelector('img.badge')!.getAttribute('title')).toBe('From Deezer')
  })
})
