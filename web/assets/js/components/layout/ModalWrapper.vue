<template>
  <DialogRoot :open @update:open="value => value || close()">
    <DialogPortal>
      <DialogOverlay class="modal-scrim" />
      <DialogContent
        :aria-describedby="undefined"
        class="modal-wrapper"
        @escape-key-down="dismiss"
        @open-auto-focus="shown"
        @pointer-down-outside="dismiss"
      >
        <DialogTitle as="span" class="sr-only">{{ title }}</DialogTitle>
        <component :is="options.component" v-if="options.component" v-bind="options.props" @close="close" />
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<script lang="ts" setup>
import { DialogContent, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from 'reka-ui'
import { useMutationObserver } from '@vueuse/core'
import { computed, ref, shallowRef, watch } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { ModalKey } from '@/config/symbols'
import { useViewport } from '@/composables/useViewport'
import { useBackToClose } from '@/composables/useBackToClose'

/**
 * The one modal dialog, showing what `useModal` opened. Reka UI runs it: focus
 * stays inside while it's open and goes back where it was after.
 */
const options = requireInjection(ModalKey)

const open = computed(() => Boolean(options.value.component))
/** The dialog's element, once shown (Reka UI tells it so: as the target of its focus on opening). */
const content = shallowRef<HTMLElement | null>(null)
const shown = (event: Event) => (content.value = event.target instanceof HTMLElement ? event.target : null)

const close = () => {
  options.value = {
    component: null,
  }
}

const { isMobile } = useViewport()

// On a phone, Back is Escape: a form with changes asks first, anything else closes.
useBackToClose(
  computed(() => open.value && isMobile.value),
  () => {
    const target = content.value?.querySelector('form') ?? content.value
    target?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
  },
)

/**
 * Escape, or a press outside, closes what only shows something (song info,
 * the equalizer). Forms answer Escape themselves, asking before unsaved
 * changes are lost.
 */
const dismiss = (event: Event) => {
  if (content.value?.querySelector('form')) {
    event.preventDefault()
  }
}

/** The dialog is named after its heading, once the component in it (often loaded on demand) shows one. */
const title = ref('')

const nameByHeading = () => (title.value = content.value?.querySelector('h1')?.textContent?.trim() ?? '')

watch(content, nameByHeading)
watch(open, isOpen => isOpen || (content.value = null))
useMutationObserver(content, nameByHeading, { childList: true, subtree: true, characterData: true })
</script>

<style lang="postcss" scoped>
.modal-scrim {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: color-mix(in srgb, var(--schemes-scrim) 32%, transparent);
}

/* koel's forms (header, main, footer) inside an M3 dialog. */
.modal-wrapper {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 1000;
  transform: translate(-50%, -50%);
  min-width: 480px;
  max-width: min(640px, calc(100vw - 48px));
  max-height: calc(100dvh - 48px);
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);
  box-shadow: var(--m3-elevation-3);

  /* On a phone, a full-screen dialog: what it says scrolls, its buttons stay at the bottom. */
  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    inset: 0;
    transform: none;
    display: flex;
    flex-direction: column;
    min-width: 0;
    max-width: none;
    max-height: none;
    border-radius: 0;
    background: var(--schemes-surface-container-low);

    :deep(> :not(.sr-only)) {
      display: flex;
      flex-direction: column;
      flex: 1;
      min-height: 0;
      width: 100% !important;

      > main {
        flex: 1;
        min-height: 0;
        overflow-y: auto;
        overscroll-behavior: contain;
        /* Room for the first field's label, which sits on its top edge. */
        padding-top: 12px;
      }
    }
  }

  &:focus-visible,
  :deep(> *:focus),
  :deep(> *:focus-visible) {
    outline: none !important;
  }

  /* The component shown; not its hidden title. */
  :deep(> :not(.sr-only)) {
    position: relative;

    > header,
    > main,
    > footer {
      padding: 0 24px;
    }

    > header {
      display: flex;
      padding-top: 24px;
      padding-bottom: 16px;

      h1 {
        margin: 0;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: var(--schemes-on-surface);
        font-size: calc(var(--static-headline-small-size) * 1px);
        line-height: calc(var(--static-headline-small-line-height) * 1px);
        font-weight: 400;
      }
    }

    > main {
      padding-bottom: 8px;
    }

    > footer {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
      padding-top: 16px;
      padding-bottom: 24px;
    }
  }
}
</style>
