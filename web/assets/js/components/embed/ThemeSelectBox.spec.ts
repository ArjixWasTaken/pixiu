import { describe, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { screen } from '@testing-library/vue'
import Component from './ThemeSelectBox.vue'

describe('themeSelectBox.vue', () => {
  const h = createHarness()

  it('renders the schemes as options', async () => {
    h.render(Component)
    await h.tick()

    await h.user.selectOptions(screen.getByRole('combobox'), ['red'])
  })
})
