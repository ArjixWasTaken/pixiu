import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import type { MailServer } from '@/services/serverSettingsService'
import Component from './MailServerForm.vue'

const saved: MailServer = {
  host: 'mail.example.com',
  port: 587,
  security: 'starttls',
  username: 'pixiu',
  password_set: true,
  from: 'píxiū <no-reply@example.com>',
}

describe('mailServerForm.vue', () => {
  const h = createHarness()

  it('sets up a mail server, the port following the security', async () => {
    const { emitted } = h.render(Component, { props: { server: null } })

    await h.type(screen.getByLabelText('Server'), 'smtp.example.com')
    await h.user.click(screen.getByRole('button', { name: 'TLS' }))
    screen.getByText(/usually port 465/)
    await h.type(screen.getByLabelText('Password'), 'secret')
    await h.type(screen.getByLabelText('Sender'), 'no-reply@example.com')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))

    expect(emitted().save).toEqual([
      [
        {
          host: 'smtp.example.com',
          port: 465,
          security: 'tls',
          username: '',
          password: 'secret',
          from: 'no-reply@example.com',
        },
      ],
    ])
    expect(screen.queryByRole('button', { name: 'Remove' })).toBeNull()
  })

  it('keeps the saved password unless told otherwise', async () => {
    const { emitted } = h.render(Component, { props: { server: saved } })

    screen.getByText('Saved. Type a new one to replace it.')
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect((emitted().save as [{ password?: string }][])[0][0].password).toBeUndefined()

    await h.user.click(screen.getByRole('button', { name: 'Forget the saved password' }))
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect((emitted().save as [{ password?: string }][])[1][0].password).toBe('')

    await h.user.click(screen.getByRole('button', { name: 'Remove' }))
    expect(emitted().remove).toHaveLength(1)
  })

  it('keeps a port chosen by hand', async () => {
    const { emitted } = h.render(Component, { props: { server: { ...saved, port: 2525 } } })

    await h.user.click(screen.getByRole('button', { name: 'None' }))
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect((emitted().save as [{ port: number }][])[0][0].port).toBe(2525)
  })
})
