import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './LoadFailedState.vue'

describe('loadFailedState.vue', () => {
  const h = createHarness()

  it('says what couldn’t load, and offers to try again', async () => {
    const { emitted } = h.render(Component, { props: { what: 'albums' } })

    screen.getByRole('alert')
    screen.getByText(/Couldn’t load the albums\./)

    await h.user.click(screen.getByRole('button', { name: 'Try again' }))
    expect(emitted().retry).toHaveLength(1)
  })
})
