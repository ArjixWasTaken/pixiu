import { describe, expect, it } from 'vite-plus/test'
import { fireEvent, screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { volumeManager } from '@/services/volumeManager'
import Component from './VolumeSlider.vue'

describe('volumeSlider.vue', () => {
  const h = createHarness({
    beforeEach: () => volumeManager.init(null, 5),
  })

  it('mutes and unmutes', async () => {
    h.render(Component)
    expect(volumeManager.volume.value).toEqual(5)

    await h.user.click(screen.getByRole('button', { name: 'Mute' }))
    expect(volumeManager.volume.value).toEqual(0)

    await h.user.click(screen.getByRole('button', { name: 'Unmute' }))
    expect(volumeManager.volume.value).toEqual(5)
  })

  it('sets the volume', async () => {
    h.render(Component)

    await fireEvent.update(screen.getByRole('slider', { name: 'Volume' }), '4.2')

    expect(volumeManager.volume.value).toBe(4.2)
  })
})
