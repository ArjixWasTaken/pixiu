import { describe, expect, it } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { defineComponent, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './Popover.vue'

/** A button that opens a panel, as the app's popovers do. */
const Host = defineComponent({
  components: { Popover: Component },
  setup: () => ({ open: ref(false) }),
  template: `
    <div>
      <Popover v-model:open="open" class="my-panel">
        <template #anchor><button type="button">Open</button></template>
        <p>panel content</p>
        <button type="button" @click="open = false">Done</button>
      </Popover>
      <p>Elsewhere</p>
    </div>
  `,
})

/** A moment after opening: Reka UI listens for clicks elsewhere from the next task on. */
const settle = () => new Promise(resolve => setTimeout(resolve))

describe('popover.vue', () => {
  const h = createHarness()

  it('opens its panel from its anchor, and says so on the anchor', async () => {
    h.render(Host)
    const anchor = screen.getByRole('button', { name: 'Open' })
    expect(screen.queryByText('panel content')).toBeNull()
    expect(anchor.getAttribute('aria-expanded')).toBe('false')

    await h.user.click(anchor)

    screen.getByText('panel content')
    expect(anchor.getAttribute('aria-expanded')).toBe('true')
    expect(screen.getByRole('dialog').classList.contains('my-panel')).toBe(true)
  })

  it('closes when its anchor is pressed again', async () => {
    h.render(Host)
    const anchor = screen.getByRole('button', { name: 'Open' })

    await h.user.click(anchor)
    await settle()
    await h.user.click(anchor)

    await waitFor(() => expect(screen.queryByText('panel content')).toBeNull())
  })

  it('closes on Escape, giving focus back to its anchor', async () => {
    h.render(Host)
    const anchor = screen.getByRole('button', { name: 'Open' })

    await h.user.click(anchor)
    await h.user.keyboard('{Escape}')

    await waitFor(() => expect(screen.queryByText('panel content')).toBeNull())
    await waitFor(() => expect(document.activeElement).toBe(anchor))
  })

  it('closes on a click elsewhere, or when told to', async () => {
    h.render(Host)

    await h.user.click(screen.getByRole('button', { name: 'Open' }))
    await settle()
    await h.user.click(screen.getByText('Elsewhere'))
    await waitFor(() => expect(screen.queryByText('panel content')).toBeNull())

    await h.user.click(screen.getByRole('button', { name: 'Open' }))
    await h.user.click(screen.getByRole('button', { name: 'Done' }))
    await waitFor(() => expect(screen.queryByText('panel content')).toBeNull())
  })
})
