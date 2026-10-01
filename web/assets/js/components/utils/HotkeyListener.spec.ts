import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import Component from './HotkeyListener.vue'

const goMock = vi.fn()
const isCurrentScreenMock = vi.fn().mockReturnValue(false)
const forwardMock = vi.fn()
const rewindMock = vi.fn()

vi.mock('@/composables/useRouter', () => ({
  useRouter: () => ({
    go: goMock,
    url: (name: string) => `/${name}`,
    isCurrentScreen: isCurrentScreenMock,
  }),
}))

vi.mock('@/services/playbackManager', () => ({
  playback: () => ({ forward: forwardMock, rewind: rewindMock }),
}))

const pressKey = (key: string) => {
  const event = new KeyboardEvent('keydown', { key, bubbles: true })
  document.body.dispatchEvent(event)
}

describe('hotkeyListener.vue', () => {
  const h = createHarness()

  it('emits FOCUS_SEARCH_FIELD on "f" key', () => {
    const emitMock = h.mock(eventBus, 'emit')

    h.render(Component)
    pressKey('f')

    expect(emitMock).toHaveBeenCalledWith('FOCUS_SEARCH_FIELD')
  })

  it('navigates to home on "h" key', () => {
    h.render(Component)
    pressKey('h')

    expect(goMock).toHaveBeenCalledWith('/home')
  })

  it('seeks forward on ArrowRight', () => {
    h.render(Component)
    pressKey('ArrowRight')

    expect(forwardMock).toHaveBeenCalledWith(10)
  })

  it('seeks backward on ArrowLeft', () => {
    h.render(Component)
    pressKey('ArrowLeft')

    expect(rewindMock).toHaveBeenCalledWith(10)
  })
})

describe('hotkeyListener.vue, where keys belong elsewhere', () => {
  const h = createHarness({
    beforeEach: () => {
      forwardMock.mockClear()
      rewindMock.mockClear()
    },
  })

  const pressOn = (target: Element, key: string, init: KeyboardEventInit = {}) =>
    target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...init }))

  it('leaves menus, dialogs, sliders and song rows their keys', () => {
    h.render(Component)

    const menu = document.createElement('div')
    menu.setAttribute('role', 'menu')
    const item = document.createElement('li')
    item.setAttribute('role', 'menuitem')
    menu.appendChild(item)

    const dialog = document.createElement('div')
    dialog.setAttribute('role', 'dialog')
    const text = document.createElement('p')
    dialog.appendChild(text)

    const slider = document.createElement('input')
    slider.type = 'range'

    const row = document.createElement('article')
    row.className = 'song-item'

    document.body.append(menu, dialog, slider, row)

    pressOn(item, 'ArrowRight')
    pressOn(text, 'ArrowLeft')
    pressOn(slider, 'ArrowRight')
    pressOn(row, 'ArrowLeft')

    expect(forwardMock).not.toHaveBeenCalled()
    expect(rewindMock).not.toHaveBeenCalled()

    menu.remove()
    dialog.remove()
    slider.remove()
    row.remove()
  })

  it('leaves Shift+arrows to selections', () => {
    h.render(Component)
    pressOn(document.body, 'ArrowRight', { shiftKey: true })

    expect(forwardMock).not.toHaveBeenCalled()
  })
})
