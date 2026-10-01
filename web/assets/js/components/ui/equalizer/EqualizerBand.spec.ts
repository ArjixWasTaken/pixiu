import { describe, expect, it } from 'vite-plus/test'
import { fireEvent, screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './EqualizerBand.vue'

describe('equalizerBand.vue', () => {
  const h = createHarness()

  it('renders with label slot', () => {
    h.render(Component, {
      props: { type: 'gain', modelValue: 0 },
      slots: { default: '1K' },
    })

    screen.getByText('1K')
  })

  it('renders slider element', () => {
    const { container } = h.render(Component, {
      props: { type: 'preamp', modelValue: 5 },
      slots: { default: 'Preamp' },
    })

    expect(container.querySelector('.slider')).toBeTruthy()
  })

  it('is a slider named by its label, that sets the gain and commits on release', async () => {
    const { emitted } = h.render(Component, {
      props: { type: 'gain', modelValue: 0 },
      slots: { default: '1K' },
    })

    const slider = screen.getByRole<HTMLInputElement>('slider', { name: '1K' })
    await fireEvent.update(slider, '4.5')
    await fireEvent.change(slider)

    expect(emitted()['update:modelValue'].at(-1)).toEqual([4.5])
    expect(emitted().commit).toBeTruthy()
  })
})
