import { describe, expect, it, vi } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { defineComponent, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import M3MenuItem from './M3MenuItem.vue'
import M3MenuPopover from './M3MenuPopover.vue'

/** What had focus when "First" was chosen. */
const chooseFirst = vi.fn(() => document.activeElement)

/** A button that opens a two-item menu, as the app's menus do. */
const Host = defineComponent({
  components: { M3MenuItem, M3MenuPopover },
  setup: () => ({ open: ref(false), chooseFirst }),
  template: `
    <div>
      <M3MenuPopover v-model:open="open">
        <template #anchor><button type="button">Options</button></template>
        <M3MenuItem label="First" @click="chooseFirst" />
        <M3MenuItem label="Second" />
      </M3MenuPopover>
      <p>Elsewhere</p>
    </div>
  `,
})

/** A moment after opening: Reka UI listens for clicks elsewhere from the next task on. */
const settle = () => new Promise(resolve => setTimeout(resolve))

describe('m3MenuPopover.vue', () => {
  const h = createHarness({
    beforeEach: () => chooseFirst.mockClear(),
  })

  it('shows its items once opened, and closes when one is chosen', async () => {
    h.render(Host)
    const anchor = screen.getByRole('button', { name: 'Options' })
    expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull()
    expect(anchor.getAttribute('aria-expanded')).toBe('false')

    await h.user.click(anchor)
    screen.getByRole('menuitem', { name: 'Second' })
    expect(anchor.getAttribute('aria-expanded')).toBe('true')

    await h.user.click(screen.getByRole('menuitem', { name: 'First' }))
    // Focus is back on the button first: a dialog the item opens gives it back there.
    expect(chooseFirst).toHaveReturnedWith(anchor)
    await waitFor(() => expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull())
  })

  it('closes on Escape, giving focus back to its button, and on a click elsewhere', async () => {
    h.render(Host)
    const anchor = screen.getByRole('button', { name: 'Options' })

    await h.user.click(anchor)
    await h.user.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull())
    await waitFor(() => expect(document.activeElement).toBe(anchor))

    await h.user.click(anchor)
    await settle()
    await h.user.click(screen.getByText('Elsewhere'))
    await waitFor(() => expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull())
  })

  it('opens from the keyboard, and the arrow keys move through it', async () => {
    h.render(Host)

    screen.getByRole('button', { name: 'Options' }).focus()
    await h.user.keyboard('{Enter}')
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'First' })))

    await h.user.keyboard('{ArrowDown}')
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Second' }))

    await h.user.keyboard('{ArrowUp}{Enter}')
    expect(chooseFirst).toHaveBeenCalled()
  })
})
