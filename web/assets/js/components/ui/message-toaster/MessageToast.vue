<template>
  <M3Snackbar
    :class="message.type"
    class="toast"
    title="Click to dismiss"
    @click="dismiss"
    @close="dismiss"
    @mouseenter="onMouseEnter"
    @mouseleave="onMouseLeave"
  >
    <span class="flex items-center gap-3">
      <M3Icon v-if="message.type !== 'info'" :name="typeIcon" :size="20" class="type-icon" />
      {{ message.content }}
    </span>
  </M3Snackbar>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, toRefs } from 'vue'

import M3Icon from '@/components/m3/M3Icon.vue'
import M3Snackbar from '@/components/m3/M3Snackbar.vue'

const props = defineProps<{ message: ToastMessage }>()
const emit = defineEmits<{ (e: 'dismiss', message: ToastMessage): void }>()

const { message } = toRefs(props)

const typeIcon = computed(() => {
  switch (message.value.type) {
    case 'info':
      return 'info'
    case 'success':
      return 'check_circle'
    case 'warning':
      return 'warning'
    default:
      return 'error'
  }
})

let timeoutHandler: number
const hovering = ref(false)

const dismiss = () => {
  emit('dismiss', message.value)
  window.clearTimeout(timeoutHandler)
}

const cancelAutoDismiss = () => window.clearTimeout(timeoutHandler)
const setAutoDismiss = () => (timeoutHandler = window.setTimeout(() => dismiss(), message.value.timeout * 1000))

const onMouseEnter = () => {
  hovering.value = true
  cancelAutoDismiss()
}

const onMouseLeave = () => {
  hovering.value = false
  setAutoDismiss()
}

onMounted(() => setAutoDismiss())
</script>

<style scoped>
.toast {
  cursor: pointer;
}

.success .type-icon {
  color: var(--schemes-inverse-primary);
}

.warning .type-icon,
.danger .type-icon {
  color: var(--schemes-error-container);
}
</style>
