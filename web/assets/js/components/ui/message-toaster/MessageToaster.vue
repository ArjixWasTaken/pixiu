<template>
  <div ref="root" class="popover">
    <TransitionGroup class="flex flex-col items-start gap-2" data-vue="MessageToaster" name="toast" tag="ul">
      <li v-for="message in messages" :key="message.id">
        <MessageToast :message="message" @dismiss="removeMessage(message)" />
      </li>
    </TransitionGroup>
  </div>
</template>

<script lang="ts" setup>
import { onMounted, ref } from 'vue'
import { uuid } from '@/utils/crypto'

import MessageToast from '@/components/ui/message-toaster/MessageToast.vue'

const root = ref<
  HTMLDivElement & {
    popover?: 'manual' | 'auto'
    showPopover?: () => void
  }
>()

const messages = ref<ToastMessage[]>([])

const addMessage = (type: 'info' | 'success' | 'warning' | 'danger', content: string, timeout = 5) => {
  root.value?.showPopover?.()

  messages.value.push({
    type,
    content,
    timeout,
    id: uuid(),
  })
}

const removeMessage = (message: ToastMessage) => {
  messages.value = messages.value.filter(({ id }) => id !== message.id)

  if (messages.value.length === 0) {
    root.value?.hidePopover?.()
  }
}

const info = (content: string, timeout?: number) => addMessage('info', content, timeout)
const success = (content: string, timeout?: number) => addMessage('success', content, timeout)
const warning = (content: string, timeout?: number) => addMessage('warning', content, timeout)
const error = (content: string, timeout?: number) => addMessage('danger', content, timeout)

onMounted(() => {
  if (!root.value) {
    return
  }

  root.value.popover = 'manual'
})

defineExpose({ info, success, warning, error })
</script>

<style lang="postcss" scoped>
/* Snackbars at the bottom left, above the player. */
.popover,
.popover:popover-open {
  inset: unset;
  position: fixed;
  left: 24px;
  bottom: 124px;
  margin: 0;
  padding: 0;
  overflow: visible;
  border: 0;
  background: transparent;

  &::backdrop {
    background: transparent;
  }

  @media (max-width: 768px) {
    left: 8px;
    right: 8px;
    bottom: 176px;
  }
}

.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 200ms linear,
    transform 200ms var(--m3-ease);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(16px);
}
</style>
