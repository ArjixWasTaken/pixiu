import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './SettingsScreen.vue'

describe('settingsScreen.vue', () => {
  const h = createHarness()

  const render = () =>
    h.render(Component, {
      global: {
        stubs: {
          AccountSettings: h.stub('account-settings'),
          YouTubeMusicSettings: h.stub('youtube-music-settings'),
          LibrarySettings: h.stub('library-settings'),
          UsersSettings: h.stub('users-settings'),
          SignInSettings: h.stub('sign-in-settings'),
          EmailSettings: h.stub('email-settings'),
        },
      },
    })

  it('shows everyone their account first, and users only to admins', () => {
    render()

    expect(screen.getAllByRole('tab').map(tab => tab.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
      'account_circle Account',
      'smart_display YouTube Music',
      'library_music Library',
    ])
  })

  it('shows admins the server tabs too', () => {
    h.actingAsAdmin()
    render()

    for (const tab of ['users', 'sign-in', 'email']) {
      screen.getByTestId(`settings-tab-${tab}`)
    }
  })
})
