import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { defineComponent, h as createElement, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './DialogBox.vue'

describe('dialogBox', () => {
  const h = createHarness()

  /** Renders the dialog box, handing back what `useDialogBox` would call. */
  const renderComponent = () => {
    const box = ref<InstanceType<typeof Component>>()
    h.render(defineComponent({ setup: () => () => createElement(Component, { ref: box }) }))
    return () => box.value!
  }

  it('shows nothing until asked', () => {
    renderComponent()

    expect(screen.queryByRole('alertdialog')).toBeNull()
  })

  it('tells, with OK only', async () => {
    const box = renderComponent()

    const answered = box().error('The file could not be read.')

    await screen.findByRole('alertdialog', { name: 'Something went wrong' })
    screen.getByText('The file could not be read.')
    expect(screen.queryByRole('button', { name: 'Cancel' })).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'OK' }))

    expect(await answered).toBe(true)
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
  })

  it('makes a bare question the headline, and focuses Cancel', async () => {
    const box = renderComponent()

    box().confirm('Discard 1 file?', { action: 'Discard' })

    await screen.findByRole('alertdialog', { name: 'Discard 1 file?' })
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Cancel' })))
  })

  it('answers false on Cancel, and on Escape', async () => {
    const box = renderComponent()

    let answered = box().confirm('Delete the playlist?', { action: 'Delete' })
    await h.user.click(await screen.findByRole('button', { name: 'Cancel' }))
    expect(await answered).toBe(false)

    answered = box().confirm('Delete the playlist?', { action: 'Delete' })
    await screen.findByRole('alertdialog')
    await h.user.keyboard('{Escape}')
    expect(await answered).toBe(false)
  })

  it('is not answered by the Escape that asked it', async () => {
    const box = renderComponent()
    let answered: Promise<boolean> | undefined
    const form = document.createElement('form')
    form.addEventListener('keydown', () => (answered ??= box().confirm('Discard all changes?', { action: 'Discard' })))
    document.body.append(form)
    const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })

    // As in a browser: the form asks, Vue renders before the next listener,
    // and the same Escape then reaches the document, where dialogs listen.
    form.dispatchEvent(escape)
    await h.tick()
    document.dispatchEvent(escape)

    await h.user.click(await screen.findByRole('button', { name: 'Discard' }))
    expect(await answered).toBe(true)
    form.remove()
  })

  it('answers true on its action, which the button names', async () => {
    const box = renderComponent()

    const answered = box().confirm('Delete the playlist?', { action: 'Delete' })
    await h.user.click(await screen.findByRole('button', { name: 'Delete' }))

    expect(await answered).toBe(true)
  })
})
