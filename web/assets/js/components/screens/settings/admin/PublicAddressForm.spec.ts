import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './PublicAddressForm.vue'

describe('publicAddressForm.vue', () => {
  const h = createHarness()

  it('offers the address in use until one is saved', async () => {
    const { emitted } = h.render(Component, { props: { url: null, suggested: 'https://music.example.com' } })

    expect(screen.getByLabelText<HTMLInputElement>('Public address').value).toBe('https://music.example.com')
    screen.getByText(/Not saved yet/)
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect(emitted().save).toEqual([['https://music.example.com']])
  })

  it('forgets the address when emptied', async () => {
    const { emitted } = h.render(Component, {
      props: { url: 'https://music.example.com', suggested: 'http://localhost:4533' },
    })

    expect(screen.queryByText(/Not saved yet/)).toBeNull()
    await h.user.clear(screen.getByLabelText('Public address'))
    await h.user.click(screen.getByRole('button', { name: 'Save' }))
    expect(emitted().save).toEqual([[null]])
  })
})
