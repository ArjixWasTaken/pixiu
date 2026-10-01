<template>
  <slot />
</template>

<script lang="ts" setup>
import type { KeyFilter } from '@vueuse/core'
import { onKeyStroke as baseOnKeyStroke } from '@vueuse/core'
import { eventBus } from '@/utils/eventBus'
import { volumeManager } from '@/services/volumeManager'
import { useQueueStore } from '@/stores/queueStore'
import { useRouter } from '@/composables/useRouter'
import { usePlayableStore } from '@/stores/playableStore'
import { playback } from '@/services/playbackManager'

const queueStore = useQueueStore()
const playableStore = usePlayableStore()

const { isCurrentScreen, go, url } = useRouter()

/**
 * Where keys belong to what has focus: fields and buttons, and widgets that
 * move with the arrow keys or act on Space and letters (menus, tabs, sliders,
 * radio groups, dialogs, song rows).
 */
const TAKES_KEYS = [
  'input',
  'select',
  'textarea',
  'button',
  'a[href]',
  '[contenteditable]',
  '[role="button"]',
  '[role="checkbox"]',
  '[role="slider"]',
  '[role="tab"]',
  '[role="radio"]',
  '[role^="menuitem"]',
  '[role="option"]',
  '.song-item',
].join(', ')

const WIDGETS = 'dialog, [role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"], [role="radiogroup"]'

const onKeyStroke = (key: KeyFilter, callback: (e: KeyboardEvent) => void) => {
  baseOnKeyStroke(key, e => {
    // Shift+arrows extend selections; other modifiers belong to the browser.
    if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || e.defaultPrevented) {
      return
    }

    const el = e.target

    if (el instanceof Element && (el.matches(TAKES_KEYS) || el.closest(WIDGETS))) {
      return
    }

    if (el instanceof HTMLElement && el.isContentEditable) {
      return
    }

    e.preventDefault()
    callback(e)
  })
}

onKeyStroke('f', () => eventBus.emit('FOCUS_SEARCH_FIELD'))
onKeyStroke('j', () => playback('current')?.playNext())
onKeyStroke('k', () => playback('current')?.playPrev())
onKeyStroke(' ', () => playback('current')?.toggle())
onKeyStroke('r', () => playback('current')?.rotateRepeatMode())
onKeyStroke('q', () => go(isCurrentScreen('Queue') ? -1 : url('queue')))
onKeyStroke('h', () => go(url('home')))

onKeyStroke('ArrowRight', () => playback('current')?.forward(10))
onKeyStroke('ArrowLeft', () => playback('current')?.rewind(10))
onKeyStroke('ArrowUp', () => volumeManager.increase())
onKeyStroke('ArrowDown', () => volumeManager.decrease())
onKeyStroke('m', () => volumeManager.toggleMute())

onKeyStroke('l', () => {
  if (!queueStore.current) {
    return
  }
  playableStore.toggleFavorite(queueStore.current)
})
</script>
