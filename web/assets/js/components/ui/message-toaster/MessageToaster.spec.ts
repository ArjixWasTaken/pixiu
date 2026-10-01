import { describe, expect, it, vi } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { defineComponent, h as createElement, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import MessageToaster from './MessageToaster.vue'

describe('messageToaster', () => {
  const h = createHarness()

  /** Renders the toaster, handing back what `useMessageToaster` would call. */
  const renderToaster = () => {
    const toaster = ref<InstanceType<typeof MessageToaster>>()
    h.render(defineComponent({ setup: () => () => createElement(MessageToaster, { ref: toaster }) }))
    return () => toaster.value!
  }

  it('shows nothing until told something', () => {
    renderToaster()

    expect(screen.getByRole('region', { name: /Notification/ }).querySelectorAll('li')).toHaveLength(0)
  })

  it('shows each message, and announces it', async () => {
    const toaster = renderToaster()

    toaster().success('Added 3 songs to the queue.')
    toaster().error('The file could not be read.')

    await waitFor(() => expect(screen.getAllByTitle('Click to dismiss')).toHaveLength(2))

    // Trouble at once, the rest in turn.
    const announcements = await screen.findAllByRole('alert', { hidden: true })
    expect(announcements.map(({ textContent }) => textContent!.replace(/\s+/g, ' ').trim())).toEqual([
      'Notification Added 3 songs to the queue.',
      'Notification The file could not be read.',
    ])
    expect(announcements.map(alert => alert.getAttribute('aria-live'))).toEqual(['polite', 'assertive'])
  })

  it('dismisses a message upon click', async () => {
    const toaster = renderToaster()
    toaster().info('Saved.')

    await h.user.click(await screen.findByTitle('Click to dismiss'))

    await waitFor(() => expect(screen.queryByTitle('Click to dismiss')).toBeNull())
  })

  it('dismisses a message after its time', async () => {
    vi.useFakeTimers()
    const toaster = renderToaster()

    toaster().info('Saved.', 2)
    await vi.advanceTimersByTimeAsync(100)
    screen.getByTitle('Click to dismiss')

    await vi.advanceTimersByTimeAsync(2000)
    expect(screen.queryByTitle('Click to dismiss')).toBeNull()

    vi.useRealTimers()
  })
})
