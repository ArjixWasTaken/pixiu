import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ThemeCard.vue'
import { themeStore } from '@/stores/themeStore'

describe('themeCard.vue', () => {
  const h = createHarness()

  it('sets the scheme when clicked', async () => {
    const theme = h.factory('theme').make({ name: 'Sample' })
    const setThemeMock = h.mock(themeStore, 'setTheme')

    h.render(Component, { props: { theme } })
    await h.user.click(screen.getByRole('button', { name: /Sample/ }))

    expect(setThemeMock).toHaveBeenCalledWith(theme)
  })
})
