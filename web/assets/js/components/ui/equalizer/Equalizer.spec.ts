import { describe, expect, it, vi } from 'vite-plus/test'
import { fireEvent, screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { equalizerStore } from '@/stores/equalizerStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { audioService } from '@/services/audioService'
import Component from './Equalizer.vue'

vi.mock('nouislider', () => ({
  default: {
    create: vi.fn((el: any) => {
      el.noUiSlider = {
        on: vi.fn(),
        set: vi.fn(),
      }
    }),
  },
}))

vi.mock('@/services/audioService', () => ({
  audioService: {
    bands: Array.from({ length: 10 }, (_, index) => ({
      label: `band-${index}`,
      db: 0,
      node: {},
    })),
    preamp: 0,
    changePreampGain: vi.fn(),
    changeFilterGain: vi.fn(),
    setBypassed: vi.fn(),
  },
}))

describe('equalizer.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      preferenceStore.temporary.equalizer_enabled = true
      h.mock(equalizerStore, 'init')
      h.mock(equalizerStore, 'getConfig').mockReturnValue({
        id: undefined,
        name: 'Default',
        preamp: 0,
        gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      })
    },
  })

  it('wires header (preset dropdown) and bands (sliders) together', () => {
    const { container } = h.render(Component)

    screen.getByText('Default')
    screen.getByText('Rock')
    screen.getByText('Preamp')
    screen.getByText('Close')
    expect(container.querySelectorAll('.slider').length).toBeGreaterThan(0)
  })

  it('emits close when the Close button is clicked', async () => {
    const { emitted } = h.render(Component)

    await fireEvent.click(screen.getByText('Close'))

    expect(emitted().close).toHaveLength(1)
  })

  it('turns off, bypassing the bands, and on again', async () => {
    h.render(Component)
    const toggle = screen.getByRole('switch', { name: 'Equalizer on' })
    screen.getByText('On')

    await h.user.click(toggle)
    expect(preferenceStore.equalizer_enabled).toBe(false)
    expect(audioService.setBypassed).toHaveBeenLastCalledWith(true)
    screen.getByText('Off')

    await h.user.click(toggle)
    expect(preferenceStore.equalizer_enabled).toBe(true)
    expect(audioService.setBypassed).toHaveBeenLastCalledWith(false)
  })
})
