import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { screen } from '@testing-library/vue'
import themes from '@/config/themes'
import { themeStore } from '@/stores/themeStore'
import Component from './ThemePreferences.vue'

describe('themePreferences.vue', () => {
  const h = createHarness()

  it('renders every scheme', () => {
    h.render(Component)

    screen.getByTestId('built-in-themes')
    expect(screen.queryAllByTestId('theme-card')).toHaveLength(themes.length)
  })

  it('switches to light mode', async () => {
    const setDarkModeMock = h.mock(themeStore, 'setDarkMode')
    h.render(Component)

    await h.user.click(screen.getByRole('button', { name: 'Light' }))

    expect(setDarkModeMock).toHaveBeenCalledWith(false)
  })
})
