<template>
  <AlertDialogRoot :open @update:open="value => value || answer(false)">
    <AlertDialogPortal>
      <AlertDialogOverlay class="dialog-scrim" />
      <AlertDialogContent :class="type" class="dialog-box">
        <div class="body">
          <M3Icon :name="icon" class="icon" />
          <AlertDialogTitle as="h3" class="m3-headline-small headline">{{ headline }}</AlertDialogTitle>
          <AlertDialogDescription as="div" class="m3-body-medium message">{{ body }}</AlertDialogDescription>
        </div>

        <footer class="actions">
          <AlertDialogCancel v-if="showCancelButton" as-child>
            <M3Button variant="text">Cancel</M3Button>
          </AlertDialogCancel>
          <M3Button variant="text" @click="answer(true)">{{ action }}</M3Button>
        </footer>
      </AlertDialogContent>
    </AlertDialogPortal>
  </AlertDialogRoot>
</template>

<script lang="ts" setup>
import {
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogOverlay,
  AlertDialogPortal,
  AlertDialogRoot,
  AlertDialogTitle,
} from 'reka-ui'
import { computed, onBeforeUnmount, ref } from 'vue'
import { useViewport } from '@/composables/useViewport'
import { useBackToClose } from '@/composables/useBackToClose'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

type DialogType = 'info' | 'success' | 'warning' | 'danger' | 'confirm'

/**
 * The app's alerts and questions (`useDialogBox`), answered with OK (true) or
 * Cancel, Escape (false). Reka UI runs it: focus stays inside, starting on
 * Cancel when there is one, and goes back where it was after.
 */
const open = ref(false)
const type = ref<DialogType>('info')
const title = ref('')
const message = ref('')

const showCancelButton = computed(() => type.value === 'confirm')

const icon = computed(
  () => ({ info: 'info', success: 'check_circle', warning: 'warning', danger: 'error', confirm: 'help' })[type.value],
)

const defaultTitle = computed(
  () => ({ info: '', success: '', warning: 'Heads up', danger: 'Something went wrong', confirm: '' })[type.value],
)

/** A question without a title is the headline itself ("Discard 1 file?"). */
const headline = computed(() => title.value || (type.value === 'confirm' ? message.value : defaultTitle.value))
const body = computed(() => (!title.value && type.value === 'confirm' ? '' : message.value))

let resolveAnswer: ((ok: boolean) => void) | null = null
let frame = 0

// On a phone, Back is Cancel.
const { isMobile } = useViewport()
useBackToClose(
  computed(() => open.value && isMobile.value),
  () => answer(false),
)

const answer = (ok: boolean) => {
  cancelAnimationFrame(frame)
  resolveAnswer?.(ok)
  resolveAnswer = null
  open.value = false
}

/** What the answering button says: the question's action (Delete, Discard…), else OK. */
const action = ref('OK')

const show = (_type: DialogType, _message: string, _title: string = '', _action = 'OK') => {
  // One at a time: a question still open goes unanswered.
  answer(false)

  type.value = _type
  message.value = _message
  title.value = _title
  action.value = _action
  // On the next frame: the key that asked (Escape in a form, "Discard all changes?") would answer it too.
  frame = requestAnimationFrame(() => (open.value = true))

  return new Promise<boolean>(resolve => (resolveAnswer = resolve))
}

onBeforeUnmount(() => cancelAnimationFrame(frame))

const success = async (message: string, title: string = '') => show('success', message, title)
const info = async (message: string, title: string = '') => show('info', message, title)
const warning = async (message: string, title: string = '') => show('warning', message, title)
const error = async (message: string, title: string = '') => show('danger', message, title)
/** A yes-or-no question; `action` names the yes ("Delete"), as the button that answers it. */
const confirm = async (message: string, { title = '', action }: { title?: string; action: string }) =>
  show('confirm', message, title, action)

defineExpose({ success, info, warning, error, confirm })
</script>

<style scoped>
.dialog-scrim {
  position: fixed;
  inset: 0;
  z-index: 1002;
  background: color-mix(in srgb, var(--schemes-scrim) 32%, transparent);
}

/* Over everything, modals included: it asks about what they're doing. */
.dialog-box {
  position: fixed;
  top: 50%;
  left: 50%;
  z-index: 1002;
  transform: translate(-50%, -50%);
  min-width: min(280px, calc(100vw - 48px));
  max-width: min(560px, calc(100vw - 48px));
  padding: 24px;
  border: 0;
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);
  box-shadow: var(--m3-elevation-3);
  outline: none;
}

.body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  text-align: center;
}

.icon {
  color: var(--schemes-secondary);

  .danger & {
    color: var(--schemes-error);
  }
}

.headline {
  color: var(--schemes-on-surface);

  &:empty {
    display: none;
  }
}

.message {
  align-self: stretch;
  text-align: start;

  &:empty {
    display: none;
  }
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 24px;
}
</style>
