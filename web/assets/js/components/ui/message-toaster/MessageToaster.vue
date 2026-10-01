<template>
  <ToastProvider label="Notification" swipe-direction="left">
    <MessageToast v-for="message in messages" :key="message.id" :message @dismiss="removeMessage(message)" />
    <ToastViewport class="message-toaster" data-vue="MessageToaster" />
  </ToastProvider>
</template>

<script lang="ts" setup>
import { ToastProvider, ToastViewport } from 'reka-ui'
import { ref } from 'vue'
import { uuid } from '@/utils/crypto'

import MessageToast from '@/components/ui/message-toaster/MessageToast.vue'

/**
 * The app's snackbars (`useMessageToaster`). Reka UI runs them: each goes after
 * its time, which stops while the pointer or focus is on them; a swipe
 * dismisses one, F8 reaches them, and screen readers hear them.
 */
const messages = ref<ToastMessage[]>([])

const addMessage = (type: 'info' | 'success' | 'warning' | 'danger', content: string, timeout = 5) =>
  messages.value.push({
    type,
    content,
    timeout,
    id: uuid(),
  })

const removeMessage = (message: ToastMessage) => {
  messages.value = messages.value.filter(({ id }) => id !== message.id)
}

const info = (content: string, timeout?: number) => addMessage('info', content, timeout)
const success = (content: string, timeout?: number) => addMessage('success', content, timeout)
const warning = (content: string, timeout?: number) => addMessage('warning', content, timeout)
const error = (content: string, timeout?: number) => addMessage('danger', content, timeout)

defineExpose({ info, success, warning, error })
</script>

<style>
/* Snackbars at the bottom left, above the player and over dialogs. (Reka UI renders the list; scoped styles can't reach it.) */
.message-toaster {
  position: fixed;
  left: 24px;
  bottom: 124px;
  z-index: 1003;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
  outline: none;

  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    left: 8px;
    right: 8px;
    bottom: 176px;
  }
}
</style>
