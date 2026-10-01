<template>
  <DropdownMenuRoot :modal="false" :open @update:open="value => value || close()">
    <DropdownMenuPortal>
      <div v-if="open && isMobile" class="sheet-scrim" />
      <DropdownMenuContent
        :class="[extraClass, { sheet: isMobile }]"
        :collision-padding="8"
        :reference="pointer"
        align="start"
        class="menu context-menu select-none"
        side="bottom"
        @close-auto-focus="returnFocus"
        @contextmenu.prevent
      >
        <span v-if="isMobile" class="sheet-handle" />
        <component :is="options.component" v-if="options.component" v-bind="options.props" />
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>

<script lang="ts" setup>
import { DropdownMenuContent, DropdownMenuPortal, DropdownMenuRoot } from 'reka-ui'
import { computed, provide, toRefs, watch } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { ContextMenuKey, ContextMenuOpenerKey } from '@/config/symbols'
import { useViewport } from '@/composables/useViewport'
import { useBackToClose } from '@/composables/useBackToClose'

/**
 * The one context menu, opened where it was asked for (`useContextMenu`).
 * Reka UI does the menu's part: focus, arrow keys, typing to an item,
 * submenus, and closing on Escape or a click outside. A click outside also
 * lands where it was aimed; on phones, the menu is a bottom sheet over a scrim.
 */
const props = defineProps<{ extraClass?: string }>()
const { extraClass } = toRefs(props)

const options = requireInjection(ContextMenuKey)
const { isMobile } = useViewport()

const open = computed(() => Boolean(options.value.component))

/** The point the menu opens at: where the pointer was, or what was given. */
const pointer = computed(() => {
  const { top, left } = options.value.position

  return {
    getBoundingClientRect: () => DOMRect.fromRect({ x: left, y: top, width: 0, height: 0 }),
  }
})

const close = () => (options.value = { component: null, position: { top: 0, left: 0 } })

// On a phone, the menu is a sheet: Back puts it away.
useBackToClose(
  computed(() => open.value && isMobile.value),
  close,
)

/** Focus goes back where it was when the menu opened (the row, the ⋮ button). */
let focusedBefore: HTMLElement | null = null

watch(open, isOpen => isOpen && (focusedBefore = document.activeElement as HTMLElement | null))

const focusOpener = () => focusedBefore?.focus?.({ preventScroll: true })

// An item gives focus back before doing its part: a dialog it opens then gives it back there too.
provide(ContextMenuOpenerKey, focusOpener)

const returnFocus = (event: Event) => {
  event.preventDefault()

  // Unless something took it meanwhile: a dialog the chosen item opened.
  const active = document.activeElement
  if (!active || active === document.body || (event.target as Node).contains(active)) {
    focusOpener()
  }

  focusedBefore = null
}
</script>

<style scoped>
/* On phones, a bottom sheet: Reka's placement gives way. */
:global([data-reka-popper-content-wrapper]:has(> .context-menu.sheet)) {
  inset: auto 0 0 0 !important;
  transform: none !important;
  min-width: 0 !important;
}

.sheet-scrim {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: color-mix(in srgb, var(--schemes-scrim) 32%, transparent);
}
</style>
