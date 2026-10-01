import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ThemeCard.vue'
import { useThemeStore } from '@/stores/themeStore'
describe('themeCard.vue', () => {
  const h = createHarness()

  it('sets the scheme when clicked', async () => {
    const theme: Theme = { id: 'red', name: 'Sample', thumbnail_color: 'rgb(255,167,155)' }
    const setThemeMock = h.mock(useThemeStore(), 'setTheme')

    h.render(Component, { props: { theme } })
    await h.user.click(screen.getByRole('button', { name: /Sample/ }))

    expect(setThemeMock).toHaveBeenCalledWith(theme)
  })
})
