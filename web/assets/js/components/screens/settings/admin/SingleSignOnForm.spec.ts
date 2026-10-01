import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { SingleSignOn } from '@/services/serverSettingsService'
import Component from './SingleSignOnForm.vue'

const saved: SingleSignOn = {
  name: 'Authelia',
  issuer: 'https://sso.example.com',
  client_id: 'pixiu',
  secret_set: true,
  scopes: [],
}

describe('singleSignOnForm.vue', () => {
  const h = createHarness()

  it('sets a provider up, with the redirect URI to give it', async () => {
    const { emitted } = h.render(Component, {
      props: { oidc: null, redirectUri: 'https://music.example.com/api/auth/oidc/callback', testing: false },
    })

    screen.getByText('https://music.example.com/api/auth/oidc/callback')
    await h.type(screen.getByLabelText('Name'), 'Authelia')
    await h.type(screen.getByLabelText('Issuer'), 'https://sso.example.com')
    await h.type(screen.getByLabelText('Client ID'), 'pixiu')
    await h.type(screen.getByLabelText('Client secret'), 'secret')
    await h.type(screen.getByLabelText('Scopes (optional)'), 'profile  email groups')
    await h.user.click(screen.getByRole('button', { name: 'Test' }))
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    const form = {
      name: 'Authelia',
      issuer: 'https://sso.example.com',
      client_id: 'pixiu',
      client_secret: 'secret',
      scopes: ['profile', 'email', 'groups'],
    }
    expect(emitted().test).toEqual([[form]])
    expect(emitted().save).toEqual([[form]])
  })

  it('keeps the saved secret, and waits for the public address', async () => {
    const { emitted } = h.render(Component, { props: { oidc: saved, redirectUri: null, testing: false } })

    screen.getByText(/Save the public address above first/)
    screen.getByText('Saved. Type a new one to replace it.')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect((emitted().save as [{ client_secret?: string }][])[0][0].client_secret).toBeUndefined()

    await h.user.click(screen.getByRole('button', { name: 'Remove' }))
    expect(emitted().remove).toHaveLength(1)
  })
})
