import { describe, expect, it, vi } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { defineComponent } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { setViewport } from '@/composables/useViewport'
import Component from './ContextMenuItem.vue'

describe('contextMenuItem', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: false }),
  })

  const renderItems = (template: string, onClick = () => {}) =>
    h.renderMenu(defineComponent({ components: { MenuItem: Component }, setup: () => ({ onClick }), template }))

  const withSubmenu = '<MenuItem>Add to<template #subMenuItems><MenuItem>Queue</MenuItem></template></MenuItem>'

  it('is a menu item named by its label', () => {
    renderItems('<MenuItem>Play</MenuItem>')

    screen.getByRole('menuitem', { name: 'Play' })
  })

  it('emits click when chosen', async () => {
    const onClick = vi.fn()
    renderItems('<MenuItem @click="onClick">Play</MenuItem>', onClick)

    await h.user.click(screen.getByRole('menuitem', { name: 'Play' }))

    expect(onClick).toHaveBeenCalled()
  })

  it('emits click on Enter', async () => {
    const onClick = vi.fn()
    renderItems('<MenuItem @click="onClick">Play</MenuItem>', onClick)

    screen.getByRole('menuitem', { name: 'Play' }).focus()
    await h.user.keyboard('{Enter}')

    expect(onClick).toHaveBeenCalled()
  })

  it('renders its icon', () => {
    renderItems('<MenuItem>Play<template #icon><span data-testid="custom-icon">I</span></template></MenuItem>')

    screen.getByTestId('custom-icon')
  })

  it('opens its submenu when chosen', async () => {
    renderItems(withSubmenu)
    const trigger = screen.getByRole('menuitem', { name: 'Add to' })

    expect(trigger.getAttribute('aria-haspopup')).toBe('menu')
    expect(screen.queryByRole('menuitem', { name: 'Queue' })).toBeNull()

    await h.user.click(trigger)

    screen.getByRole('menuitem', { name: 'Queue' })
  })

  it('opens its submenu with the right arrow key', async () => {
    renderItems(withSubmenu)

    screen.getByRole('menuitem', { name: 'Add to' }).focus()
    await h.user.keyboard('{ArrowRight}')

    screen.getByRole('menuitem', { name: 'Queue' })
  })

  it('opens its submenu in place in the phone sheet', async () => {
    setViewport({ mobile: true })
    renderItems(withSubmenu)
    const trigger = screen.getByRole('menuitem', { name: 'Add to' })

    expect(trigger.getAttribute('aria-expanded')).toBe('false')

    await h.user.click(trigger)

    expect(trigger.getAttribute('aria-expanded')).toBe('true')
    screen.getByRole('menuitem', { name: 'Queue' })
  })
})
