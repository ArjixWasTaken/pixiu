import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './AddWatchForm.vue'

describe('addWatchForm.vue', () => {
  const h = createHarness()

  it('offers the choice of releases for artists on either platform', async () => {
    h.render(Component)
    const field = screen.getByRole('textbox', { name: 'Playlist or artist link' })

    for (const link of [
      'https://music.youtube.com/channel/UCabcdefghijklmnopqrstuv',
      'https://www.deezer.com/en/artist/27',
    ]) {
      await h.user.clear(field)
      await h.user.type(field, link)
      screen.getByText('Singles and EPs too')
    }

    await h.user.clear(field)
    await h.user.type(field, 'https://www.deezer.com/playlist/908622995')
    expect(screen.queryByText('Singles and EPs too')).toBeNull()
  })
})
