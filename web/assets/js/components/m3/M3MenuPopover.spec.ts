import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { defineComponent, ref } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import M3MenuPopover from './M3MenuPopover.vue'

/** A button that opens a two-item menu, as the app's menus do. */
const Host = defineComponent({
  components: { M3MenuPopover },
  setup: () => ({ open: ref(false) }),
  template: `
    <div>
      <M3MenuPopover v-model:open="open">
        <template #anchor><button @click="open = !open">Options</button></template>
        <div role="menuitem" @click="open = false">First</div>
        <div role="menuitem">Second</div>
      </M3MenuPopover>
      <p>Elsewhere</p>
    </div>
  `,
})

describe('m3MenuPopover.vue', () => {
  const h = createHarness()

  it('shows its items once opened, and closes when one is chosen', async () => {
    h.render(Host)
    expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Options' }))
    screen.getByRole('menuitem', { name: 'Second' })

    await h.user.click(screen.getByRole('menuitem', { name: 'First' }))
    expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull()
  })

  it('closes on Escape and on a click elsewhere', async () => {
    h.render(Host)

    await h.user.click(screen.getByRole('button', { name: 'Options' }))
    await h.user.keyboard('{Escape}')
    expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull()

    await h.user.click(screen.getByRole('button', { name: 'Options' }))
    await h.user.click(screen.getByText('Elsewhere'))
    expect(screen.queryByRole('menuitem', { name: 'First' })).toBeNull()
  })
})
