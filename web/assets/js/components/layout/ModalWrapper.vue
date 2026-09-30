<template>
  <dialog
    ref="dialog"
    class="modal-wrapper m-auto min-w-full md:min-w-[480px] border-0 p-0 overflow-visible"
    @cancel.prevent="onEscape"
    @close.prevent
    @keydown.esc.prevent="onEscape"
  >
    <component :is="options.component" v-if="options.component" v-bind="props" @close="close" />
  </dialog>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from 'vue'
import { requireInjection } from '@/utils/helpers'
import { ModalKey } from '@/config/symbols'

const dialog = ref<HTMLDialogElement>()
const options = requireInjection(ModalKey)

const toggleCssClass = (...classes: string[]) => classes.forEach(c => dialog.value?.classList.toggle(c))

const props = computed(() => ({
  ...(options.value.props || {}),
  toggleCssClass:
    options.value.props && 'toggleCssClass' in options.value.props
      ? options.value.props.toggleCssClass
      : toggleCssClass,
}))

const close = () => {
  options.value = {
    component: null,
  }
}

/**
 * Escape closes what only shows something (song info, the equalizer). Forms
 * answer it themselves, asking before unsaved changes are lost.
 */
const onEscape = (event: Event) => {
  const target = event.target instanceof Element ? event.target : null
  if (target?.closest('form') || dialog.value?.querySelector('form')) {
    return
  }
  close()
}

watch(
  () => options.value.component,
  component => (component ? dialog.value?.showModal() : dialog.value?.close()),
)
</script>

<style lang="postcss" scoped>
/* koel's forms (header, main, footer) inside an M3 dialog. */
.modal-wrapper {
  max-width: min(640px, calc(100vw - 48px));
  max-height: calc(100dvh - 48px);
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);
  box-shadow: var(--m3-elevation-3);

  &::backdrop {
    background: color-mix(in srgb, var(--schemes-scrim) 32%, transparent);
  }

  @media (max-width: 768px) {
    max-width: 100vw;
    max-height: 100dvh;
    border-radius: 0;
  }

  &:focus-visible,
  :deep(> *:focus),
  :deep(> *:focus-visible) {
    outline: none !important;
  }

  :deep(> *) {
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
