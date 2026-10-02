<template>
  <div :class="{ loading }" class="audio-player" data-vue="AudioPlayer">
    <span class="time m3-label-medium text-right">{{ current }}</span>
    <M3Slider
      v-model="position"
      :key-step="5"
      :max="duration || 1"
      :step="0.1"
      :value-text="`${current} of ${total}`"
      class="slider"
      label="Seek"
    />
    <span class="time m3-label-medium">{{ total }}</span>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { usePlaybackProgress } from '@/composables/usePlaybackProgress'

import M3Slider from '@/components/m3/M3Slider.vue'

const { currentTime, duration, loading, current, total, seek } = usePlaybackProgress()

const position = computed({
  get: () => currentTime.value,
  set: seek,
})
</script>

<style scoped>
.audio-player {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  max-width: 640px;
}

.time {
  width: 40px;
  flex-shrink: 0;
  color: var(--schemes-on-surface-variant);
  font-variant-numeric: tabular-nums;
}

.slider {
  flex: 1;
  min-width: 0;
}

.loading .slider {
  opacity: 0.6;
}
</style>
