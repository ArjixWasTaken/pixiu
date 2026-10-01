import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './SettingsScreen.vue'

describe('settingsScreen.vue', () => {
  const h = createHarness()

  const render = () =>
    h.render(Component, {
      global: {
        stubs: {
          AccountSettings: h.stub('account-settings'),
          PreferencesSettings: h.stub('preferences-settings'),
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
      'Account',
      'Preferences',
      'YouTube Music',
      'Library',
    ])
    expect(screen.queryByText('Server')).toBeNull()
  })

  it('shows admins the server tabs too', () => {
    h.actingAsAdmin()
    render()

    for (const tab of ['admin-users', 'admin-sign-in', 'admin-email']) {
      screen.getByTestId(`settings-tab-${tab}`)
    }

    // After a label of their own.
    screen.getByText('Server')
  })

  it('opens the tab the hash names', async () => {
    h.actingAsAdmin()
    await h.visit('/settings#admin-users')
    render()

    expect(screen.getByTestId('settings-tab-admin-users').getAttribute('aria-selected')).toBe('true')
  })

  it('keeps the chosen tab in the hash', async () => {
    render()

    await h.user.click(screen.getByRole('tab', { name: 'Preferences' }))

    await waitFor(() => expect(h.router.currentRoute.value.hash).toBe('#preferences'))
    expect(screen.getByTestId('settings-tab-preferences').getAttribute('aria-selected')).toBe('true')
  })

  it('takes a link from before, with the tab in the query, to the hash', async () => {
    h.actingAsAdmin()
    await h.visit('/settings?tab=users')
    render()

    expect(h.router.currentRoute.value.fullPath).toBe('/settings#admin-users')
    screen.getByRole('tab', { name: 'Users', selected: true })
  })
})
