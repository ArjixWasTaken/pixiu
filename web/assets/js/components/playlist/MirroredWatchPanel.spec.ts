import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { PlaylistWatch } from '@/services/huntingService'
import Component from './MirroredWatchPanel.vue'

const mirror: PlaylistWatch = {
  watch: {
    id: 1,
    kind: 'liked_music',
    name: 'Liked music',
    link: 'https://music.youtube.com/playlist?list=LM',
    last_synced_at: null,
  },
  coming: [
    { key: 'youtube_music:a', title: 'Unsynced', artist: 'Somebody', job: null },
    { key: 'youtube_music:b', title: 'Queued', artist: 'Somebody', job: { state: 'queued', error: null } },
    { key: 'youtube_music:c', title: 'Running', artist: 'Somebody', job: { state: 'running', error: null } },
    {
      key: 'youtube_music:d',
      title: 'Broken',
      artist: 'Somebody',
      job: { state: 'failed', error: 'This video is unavailable' },
    },
    { key: 'youtube_music:e', title: 'Landed', artist: 'Somebody', job: { state: 'done', error: null } },
  ],
  excluded: [],
}

describe('mirroredWatchPanel.vue', () => {
  const h = createHarness()

  it('says how each coming song is doing, and where the downloads are', async () => {
    h.render(Component, { props: { mirror } })

    await h.user.click(screen.getByRole('button', { name: '5 songs still coming' }))

    screen.getByText('Somebody · waiting for the next sync')
    screen.getByText('Somebody · waiting to download')
    screen.getByText('Somebody · downloading')
    screen.getByText('Somebody · failed: This video is unavailable')
    screen.getByText('Somebody · downloaded; waiting for the next sync')
    expect(screen.getByRole('link', { name: 'See the downloads on Jobs' }).getAttribute('href')).toMatch(/jobs$/)
  })
})
