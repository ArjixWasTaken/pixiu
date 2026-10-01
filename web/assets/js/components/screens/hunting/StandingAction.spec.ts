import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './StandingAction.vue'

describe('standingAction.vue', () => {
  const h = createHarness()

  it('links a result the library holds to its album', () => {
    h.render(Component, { props: { standing: 'hoarded', libraryAlbum: 'al-7' } })

    expect(screen.getByRole('link', { name: /In your library/ }).getAttribute('href')).toMatch(/albums\/al-7$/)
  })

  it('says so without a link when the album is unknown', () => {
    h.render(Component, { props: { standing: 'hoarded' } })

    screen.getByText('In your library')
    expect(screen.queryByRole('link')).toBeNull()
  })

  it('offers a download for what the library lacks', async () => {
    const { emitted } = h.render(Component, { props: { standing: 'missing' } })

    await h.user.click(screen.getByRole('button', { name: 'Download' }))
    expect(emitted().grab).toBeTruthy()
  })
})
