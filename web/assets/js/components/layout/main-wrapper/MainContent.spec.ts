import { ref } from 'vue'
import { screen } from '@testing-library/vue'
import { describe, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { CurrentStreamableKey } from '@/config/symbols'
import Component from './MainContent.vue'

describe('mainContent.vue', () => {
  const h = createHarness()

  it('has the search bar and the current screen', async () => {
    h.render(Component, {
      global: {
        provide: {
          [<symbol>CurrentStreamableKey]: ref(h.factory('song').make()),
        },
        stubs: {
          HomeScreen: h.stub('home-screen'),
          ProfileDropdown: h.stub('profile-dropdown'),
        },
      },
    })

    screen.getByRole('search')
    await screen.findByTestId('home-screen')
  })
})
