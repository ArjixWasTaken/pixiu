import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './RegistrationSwitch.vue'

describe('registrationSwitch.vue', () => {
  const h = createHarness()

  it('opens and closes registration', async () => {
    const { emitted } = h.render(Component, { props: { open: false, mailReady: true } })

    await h.user.click(screen.getByRole('checkbox'))
    expect(emitted().toggle).toEqual([[true]])
    expect(screen.queryByText(/Needs email/)).toBeNull()
  })

  it('waits for email', () => {
    h.render(Component, { props: { open: true, mailReady: false } })

    const box = screen.getByRole<HTMLInputElement>('checkbox')
    expect(box.disabled).toBe(true)
    expect(box.checked).toBe(false)
    screen.getByText(/Needs email/)
  })
})
