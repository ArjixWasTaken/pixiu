<template>
  <span id="volume" :class="level" class="volume">
    <M3IconButton
      :icon="level === 'muted' ? 'volume_off' : level === 'discreet' ? 'volume_down' : 'volume_up'"
      :label="level === 'muted' ? 'Unmute' : 'Mute'"
      @click="level === 'muted' ? unmute() : mute()"
    />
    <M3Slider v-model="volume" :max="10" :step="0.1" class="slider" label="Volume" />
  </span>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { volumeManager } from '@/services/volumeManager'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Slider from '@/components/m3/M3Slider.vue'

const level = computed(() => {
  if (volumeManager.volume.value === 0) {
    return 'muted'
  }
  if (volumeManager.volume.value < 3) {
    return 'discreet'
  }
  return 'loud'
})

const mute = () => volumeManager.mute()
const unmute = () => volumeManager.unmute()
const volume = computed({
  get: () => volumeManager.volume.value,
  set: value => volumeManager.set(value),
})
</script>

<style scoped>
.volume {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.volume > .slider {
  flex: 0 0 auto;
  width: 112px;
}
</style>
