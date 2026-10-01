import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { DialogBoxStub } from '@/__tests__/stubs'
import { adminService } from '@/services/adminService'
import Component from './AddAccountForm.vue'

describe('addAccountForm.vue', () => {
  const h = createHarness()

  it('makes an account', async () => {
    const create = h.mock(adminService, 'createUser').mockResolvedValue(undefined)
    const { emitted } = h.render(Component)

    await h.user.type(screen.getByRole('textbox', { name: 'Username' }), 'dave')
    await h.user.type(screen.getByLabelText('Temporary password'), 'temporary1')
    await h.user.click(screen.getByRole('button', { name: /Make account/ }))

    await waitFor(() => expect(emitted().created).toBeTruthy())
    expect(create).toHaveBeenCalledWith({ username: 'dave', email: '', password: 'temporary1', role: 'user' })
  })

  it('cancels straight away while untouched', async () => {
    const confirm = h.mock(DialogBoxStub.value, 'confirm')
    const { emitted } = h.render(Component)

    await h.user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(emitted().cancel).toBeTruthy()
    expect(confirm).not.toHaveBeenCalled()
  })

  it('asks before throwing away what was typed', async () => {
    const confirm = h.mock(DialogBoxStub.value, 'confirm', false)
    const { emitted } = h.render(Component)

    await h.user.type(screen.getByRole('textbox', { name: 'Username' }), 'dave')
    await h.user.click(screen.getByRole('button', { name: 'Cancel' }))

    expect(confirm).toHaveBeenCalledWith('Discard this account?')
    expect(emitted().cancel).toBeUndefined()
  })
})
