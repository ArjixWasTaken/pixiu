import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { screen } from '@testing-library/vue'
import themes from '@/config/themes'
import Component from './ThemeList.vue'

describe('themeList.vue', () => {
  const h = createHarness()

  it('renders a list of themes', async () => {
    h.render(Component, {
      props: {
        themes,
      },
      global: {
        stubs: {
          ThemeCard: h.stub('theme-card'),
        },
      },
    })

    expect(screen.queryAllByTestId('theme-card')).toHaveLength(themes.length)
  })
})
