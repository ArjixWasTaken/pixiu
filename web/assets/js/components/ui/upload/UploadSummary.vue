<template>
  <section class="flex flex-col gap-2" data-testid="upload-summary">
    <p class="m3-body-medium self-end text-(--schemes-on-surface-variant) tabular-nums">
      {{ formatBytes(sentBytes) }} of {{ formatBytes(totalBytes) }} uploaded
      <span v-if="secondsLeft !== null" :data-seconds="Math.round(secondsLeft)" data-testid="time-left">
        · about {{ secondsToHumanReadable(secondsLeft) }} left
      </span>
    </p>
    <progress :max="totalBytes || 1" :value="sentBytes" class="bar" data-testid="upload-progress" />
  </section>
</template>

<script lang="ts" setup>
import { formatBytes, secondsToHumanReadable } from '@/utils/formatters'
import { useUploadProgress } from '@/composables/useUploadProgress'

const { totalBytes, sentBytes, secondsLeft } = useUploadProgress()
</script>

<style scoped>
.bar {
  width: 100%;
  height: 4px;
  overflow: hidden;
  border: 0;
  border-radius: 2px;
  appearance: none;
  background: var(--schemes-secondary-container);

  &::-moz-progress-bar {
    background: var(--schemes-primary);
  }

  &::-webkit-progress-bar {
    background: var(--schemes-secondary-container);
  }

  &::-webkit-progress-value {
    background: var(--schemes-primary);
  }
}
</style>
