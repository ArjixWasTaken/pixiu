<template>
  <ToastRoot v-model:open="open" :duration="message.timeout * 1000" :type class="message-toast">
    <M3Snackbar
      :class="message.type"
      title="Click to dismiss"
      @click="open = false"
      @close="open = false"
      @vue:unmounted="emit('dismiss', message)"
    >
      <span class="flex items-center gap-3">
        <M3Icon v-if="message.type !== 'info'" :name="typeIcon" :size="20" class="type-icon" />
        {{ message.content }}
      </span>
    </M3Snackbar>
  </ToastRoot>
</template>

<script lang="ts" setup>
import { ToastRoot } from 'reka-ui'
import { computed, ref, toRefs } from 'vue'

import M3Icon from '@/components/m3/M3Icon.vue'
import M3Snackbar from '@/components/m3/M3Snackbar.vue'

const props = defineProps<{ message: ToastMessage }>()
/** Once it's gone from the screen, its closing animation included. */
const emit = defineEmits<{ (e: 'dismiss', message: ToastMessage): void }>()

const { message } = toRefs(props)

const open = ref(true)

/** Trouble is announced at once; the rest waits its turn. */
const type = computed(() => (['warning', 'danger'].includes(message.value.type) ? 'foreground' : 'background'))

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
</script>

<style scoped>
.success .type-icon {
  color: var(--schemes-inverse-primary);
}

.warning .type-icon,
.danger .type-icon {
  color: var(--schemes-error-container);
}
</style>

<style>
/* The toast's own element is Reka UI's: scoped styles can't reach it. */
.message-toast {
  cursor: pointer;
  /* Clickable while a dialog keeps the rest of the page from the pointer. */
  pointer-events: auto;
  outline: none;

  &[data-state='open'] {
    animation: message-toast-in 200ms var(--m3-ease);
  }

  &[data-state='closed'] {
    animation: message-toast-out 150ms linear;
  }

  &[data-swipe='move'] {
    transform: translateX(var(--reka-toast-swipe-move-x));
  }

  &[data-swipe='cancel'] {
    transform: translateX(0);
    transition: transform 200ms var(--m3-ease);
  }

  &[data-swipe='end'] {
    animation: message-toast-swipe-out 150ms ease-out;
  }

  &:focus-visible .m3-snackbar {
    outline: 2px solid var(--schemes-inverse-primary);
    outline-offset: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    animation: none !important;
  }
}

@keyframes message-toast-in {
  from {
    opacity: 0;
    transform: translateY(16px);
  }
}

@keyframes message-toast-out {
  to {
    opacity: 0;
  }
}

@keyframes message-toast-swipe-out {
  from {
    transform: translateX(var(--reka-toast-swipe-end-x));
  }

  to {
    transform: translateX(-100%);
    opacity: 0;
  }
}
</style>
