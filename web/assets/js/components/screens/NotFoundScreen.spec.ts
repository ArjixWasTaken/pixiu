import { screen } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './NotFoundScreen.vue'

describe('NotFoundScreen', () => {
  const h = createHarness()

  it('says there is nothing here, and leads home', () => {
    h.render(Component)

    screen.getByText(/There’s nothing here/)
    expect(screen.getByRole('link', { name: /Go to Home/ }).getAttribute('href')).toBe('/home')
  })
})
