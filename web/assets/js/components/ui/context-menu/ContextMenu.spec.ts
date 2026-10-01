import { describe, expect, it, vi } from 'vite-plus/test'
import { screen, waitFor } from '@testing-library/vue'
import { defineComponent, shallowRef } from 'vue'
import type { Component as VueComponent } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { ContextMenuKey } from '@/config/symbols'
import { setViewport } from '@/composables/useViewport'
import MenuItem from './ContextMenuItem.vue'
import Component from './ContextMenu.vue'

describe('contextMenu', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: false }),
  })

  /** What had focus when "Play" was chosen. */
  const play = vi.fn(() => document.activeElement)

  const SongMenu = defineComponent({
    components: { MenuItem },
    setup: () => ({ play }),
    template: '<ul role="none"><MenuItem @click="play">Play</MenuItem><MenuItem>Delete</MenuItem></ul>',
  })

  const renderContextMenu = (props: { extraClass?: string } = {}) => {
    const options = shallowRef<{ component: VueComponent | null; position: { top: number; left: number } }>({
      component: null,
      position: { top: 0, left: 0 },
    })

    h.render(Component, { props, global: { provide: { [ContextMenuKey as symbol]: options } } })

    const open = async (position = { top: 100, left: 200 }) => {
      options.value = { component: SongMenu, position }
      await h.tick(2)
    }

    return { options, open }
  }

  it('is closed until a menu is asked for', () => {
    renderContextMenu()

    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('opens the menu asked for, at the pointer, with focus', async () => {
    const { open } = renderContextMenu()

    await open({ top: 100, left: 200 })

    const menu = screen.getByRole('menu')
    screen.getByRole('menuitem', { name: 'Play' })
    await waitFor(() => expect(menu.parentElement!.style.transform).toBe('translate(200px, 100px)'))
    await waitFor(() => expect(menu.contains(document.activeElement)).toBe(true))
  })

  it('closes when an item is chosen', async () => {
    const { options, open } = renderContextMenu()
    await open()

    await h.user.click(screen.getByRole('menuitem', { name: 'Play' }))

    await waitFor(() => expect(options.value.component).toBeNull())
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull())
  })

  it('closes on Escape and gives focus back', async () => {
    const row = document.createElement('button')
    document.body.append(row)
    row.focus()

    const { options, open } = renderContextMenu()
    await open()
    await waitFor(() => expect(screen.getByRole('menu').contains(document.activeElement)).toBe(true))

    await h.user.keyboard('{Escape}')

    await waitFor(() => expect(options.value.component).toBeNull())
    await waitFor(() => expect(document.activeElement).toBe(row))
    row.remove()
  })

  it('gives focus back to its opener before an item does its part, so a dialog it opens gives it back there', async () => {
    const row = document.createElement('button')
    document.body.append(row)
    row.focus()

    const { open } = renderContextMenu()
    await open()
    await waitFor(() => expect(screen.getByRole('menu').contains(document.activeElement)).toBe(true))

    await h.user.click(screen.getByRole('menuitem', { name: 'Play' }))

    expect(play).toHaveReturnedWith(row)
    row.remove()
  })

  it('closes when the menu is cleared', async () => {
    const { options, open } = renderContextMenu()
    await open()

    options.value = { component: null, position: { top: 0, left: 0 } }

    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull())
  })

  it('applies extra class', async () => {
    const { open } = renderContextMenu({ extraClass: 'my-custom-class' })
    await open()

    expect(screen.getByRole('menu').classList.contains('my-custom-class')).toBe(true)
  })

  it('is a bottom sheet over a scrim on phones', async () => {
    setViewport({ mobile: true })
    const { open } = renderContextMenu()
    await open()

    expect(screen.getByRole('menu').classList.contains('sheet')).toBe(true)
    expect(document.querySelector('.sheet-scrim')).not.toBeNull()
  })
})
