import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './AudioPlayer.vue'

describe('audioPlayer', () => {
  const h = createHarness()

  it('renders the seek slider with the times', () => {
    const { container } = h.render(Component)

    screen.getByRole('slider', { name: 'Seek' })
    expect(container.querySelectorAll('.time')).toHaveLength(2)
  })
})
