<template>
  <div class="m3-snackbar">
    <span class="m3-body-medium message"><slot /></span>
    <slot name="action" />
    <button v-if="closeable" aria-label="Close" class="close m3-state" type="button" @click="emit('close')">
      <M3Icon name="close" />
    </button>
  </div>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

withDefaults(defineProps<{ closeable?: boolean }>(), { closeable: true })

const emit = defineEmits<{ (e: 'close'): void }>()
</script>

<style scoped>
.m3-snackbar {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 48px;
  max-width: min(560px, calc(100vw - 32px));
  padding: 4px 4px 4px 16px;
  border-radius: 4px;
  background: var(--schemes-inverse-surface);
  color: var(--schemes-inverse-on-surface);
  box-shadow: var(--m3-elevation-3);
  box-sizing: border-box;

  :deep(.m3-button.text) {
    color: var(--schemes-inverse-primary);
  }
}

.message {
  flex: 1;
  padding: 10px 0;
}

.close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border: 0;
  border-radius: 50%;
  background: transparent;
  color: var(--schemes-inverse-on-surface);
  cursor: pointer;
}
</style>
