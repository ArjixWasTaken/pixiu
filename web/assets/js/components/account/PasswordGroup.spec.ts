import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { accountService } from '@/services/accountService'
import Component from './PasswordGroup.vue'

describe('passwordGroup.vue', () => {
  const h = createHarness()

  it('changes the password with the current one', async () => {
    const changeMock = h.mock(accountService, 'changePassword').mockResolvedValue(undefined)
    const { emitted } = h.render(Component)

    await h.type(screen.getByLabelText('Current password'), 'old secret')
    await h.type(screen.getByLabelText('New password'), 'new secret!')
    await h.type(screen.getByLabelText('New password again'), 'new secret!')
    await h.user.click(screen.getByRole('button', { name: 'Change password' }))

    expect(changeMock).toHaveBeenCalledWith('new secret!', 'old secret')
    expect(emitted().changed).toHaveLength(1)
  })

  it('does not send passwords that differ', async () => {
    const changeMock = h.mock(accountService, 'changePassword')
    h.render(Component)

    await h.type(screen.getByLabelText('Current password'), 'old secret')
    await h.type(screen.getByLabelText('New password'), 'new secret!')
    await h.type(screen.getByLabelText('New password again'), 'something else')
    screen.getByText('The passwords differ.')
    await h.user.click(screen.getByRole('button', { name: 'Change password' }))

    expect(changeMock).not.toHaveBeenCalled()
  })

  it('does not ask for a temporary password back', () => {
    h.render(Component, { props: { temporary: true } })
    expect(screen.queryByLabelText('Current password')).toBeNull()
  })
})
