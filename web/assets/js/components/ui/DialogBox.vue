<template>
  <dialog ref="dialog" :class="`${type}`" class="dialog-box">
    <div class="body">
      <M3Icon :name="icon" class="icon" />
      <h3 class="m3-headline-small headline">{{ headline }}</h3>
      <div v-if="body" class="m3-body-medium message">{{ body }}</div>
    </div>

    <footer class="actions">
      <M3Button v-if="showCancelButton" name="cancel" variant="text" @click.prevent="cancel">Cancel</M3Button>
      <M3Button name="ok" variant="text">OK</M3Button>
    </footer>
  </dialog>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

type DialogType = 'info' | 'success' | 'warning' | 'danger' | 'confirm'

const dialog = ref<HTMLDialogElement>()
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

// @ts-ignore
const close = () => dialog.value?.close()
const cancel = () => dialog.value?.dispatchEvent(new Event('cancel'))

const waitForInput = () =>
  new Promise(resolve => {
    dialog.value?.addEventListener(
      'cancel',
      () => {
        close()
        resolve(false)
      },
      { once: true },
    )

    dialog.value?.querySelector('[name=ok]')!.addEventListener(
      'click',
      () => {
        close()
        resolve(true)
      },
      { once: true },
    )
  })

const show = async (_type: DialogType, _message: string, _title: string = '') => {
  type.value = _type
  message.value = _message
  title.value = _title

  // @ts-ignore
  dialog.value.showModal()

  return waitForInput()
}

const success = async (message: string, title: string = '') => show('success', message, title)
const info = async (message: string, title: string = '') => show('info', message, title)
const warning = async (message: string, title: string = '') => show('warning', message, title)
const error = async (message: string, title: string = '') => show('danger', message, title)
const confirm = async (message: string, title: string = '') => show('confirm', message, title)

defineExpose({ success, info, warning, error, confirm })
</script>

<style scoped>
.dialog-box {
  margin: auto;
  min-width: min(280px, calc(100vw - 48px));
  max-width: min(560px, calc(100vw - 48px));
  padding: 24px;
  border: 0;
  border-radius: 28px;
  background: var(--schemes-surface-container-high);
  color: var(--schemes-on-surface-variant);
  box-shadow: var(--m3-elevation-3);

  &::backdrop {
    background: color-mix(in srgb, var(--schemes-scrim) 32%, transparent);
  }
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
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding-top: 24px;
}
</style>
