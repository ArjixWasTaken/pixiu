import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ViewModeSwitch.vue'

describe('viewModeSwitch.vue', () => {
  const h = createHarness()

  it.each<[ViewMode, string, string]>([
    ['grid', 'Grid', 'List'],
    ['list', 'List', 'Grid'],
  ])('marks %s as pressed', (mode, pressed, unpressed) => {
    h.render(Component, { props: { modelValue: mode } })

    expect(screen.getByRole('button', { name: pressed }).getAttribute('aria-pressed')).toBe('true')
    expect(screen.getByRole('button', { name: unpressed }).getAttribute('aria-pressed')).toBe('false')
  })

  it('emits the correct event', async () => {
    const { emitted } = h.render(Component, { props: { modelValue: 'grid' } })

    await h.user.click(screen.getByRole('button', { name: 'List' }))
    expect(emitted()['update:modelValue'][0]).toEqual(['list'])

    await h.user.click(screen.getByRole('button', { name: 'Grid' }))
    expect(emitted()['update:modelValue'][1]).toEqual(['grid'])
  })
})
