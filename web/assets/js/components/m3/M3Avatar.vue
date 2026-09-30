<template>
  <span :style="{ width: `${size}px`, height: `${size}px` }" class="m3-avatar">
    <img v-if="src" :src alt="" />
    <span v-else class="m3-title-medium">{{ initials }}</span>
  </span>
</template>

<script lang="ts" setup>
import { computed } from 'vue'

const props = withDefaults(defineProps<{ src?: string | null; name?: string; size?: number }>(), { size: 40 })

const initials = computed(() => (props.name ?? '').trim().charAt(0).toUpperCase())
</script>

<style scoped>
.m3-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  border-radius: 50%;
  background: var(--schemes-primary-container);
  color: var(--schemes-on-primary-container);

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
}
</style>
