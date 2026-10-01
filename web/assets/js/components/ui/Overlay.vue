<template>
  <dialog
    ref="el"
    :class="state.type"
    class="m-auto border-0 p-0 bg-transparent backdrop:bg-black/80 outline-hidden"
    data-testid="overlay"
    @cancel.prevent="onCancel"
  >
    <span class="flex items-baseline justify-center gap-3">
      <SoundBars v-if="state.type === 'loading'" />
      <M3Icon v-if="state.type === 'error'" name="error" />
      <M3Icon v-if="state.type === 'warning'" name="warning" />
      <M3Icon v-if="state.type === 'info'" name="info" />
      <M3Icon v-if="state.type === 'success'" name="check_circle" />

      <span class="message" v-html="state.message" />
    </span>
  </dialog>
</template>

<script lang="ts" setup>
import { defineAsyncComponent, reactive, ref } from 'vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const SoundBars = defineAsyncComponent(() => import('@/components/ui/SoundBars.vue'))

const el = ref<HTMLDialogElement>()

const state = reactive<OverlayState>({
  dismissible: false,
  type: 'loading',
  message: '',
})

const show = (options: Partial<OverlayState> = {}) => {
  Object.assign(state, options)
  el.value?.open || el.value?.showModal()
}

const hide = () => el.value?.close()
const onCancel = () => state.dismissible && hide()

defineExpose({ show, hide })
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
dialog {
  /* since the texts are placed directly on a dark backdrop, the colors should be a bit washed out */

  &.error {
    @apply text-red-400;
  }

  &.success {
    @apply text-green-400;
  }

  &.info {
    @apply text-blue-400;
  }

  &.warning {
    @apply text-orange-400;
  }
}
</style>
