<template>
  <span class="flex transition-opacity duration-200">
    <button class="rounded-l! border-r-0!" title="Zoom out" type="button" @click.prevent="$emit('out')">
      <M3Icon name="zoom_out" />
    </button>
    <button class="rounded-r!" title="Zoom in" type="button" @click.prevent="$emit('in')">
      <M3Icon name="zoom_in" />
    </button>
  </span>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'

defineEmits<{ (e: 'in' | 'out'): void }>()
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
button {
  @apply text-(--schemes-on-surface) bg-(--schemes-surface-container) border border-solid border-(--schemes-outline-variant) opacity-80
  px-3 py-1.5 hover:opacity-100 active:scale-95;
}
</style>
