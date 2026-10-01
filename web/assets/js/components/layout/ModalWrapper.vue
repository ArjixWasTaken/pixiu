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

  @media (max-width: 768px) {
    min-width: 100vw;
    max-width: 100vw;
    max-height: 100dvh;
    border-radius: 0;
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
