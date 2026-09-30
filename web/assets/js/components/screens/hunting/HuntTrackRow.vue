<template>
  <M3ListItem
    :supporting="
      [track.artist, track.album, track.length ? secondsToHis(track.length) : null].filter(Boolean).join(' · ')
    "
    class="rounded-2xl"
  >
    <template #leading>
      <span :style="track.cover ? { backgroundImage: `url(${track.cover})` } : {}" class="cover" />
    </template>
    <template #headline>
      <span :title="track.title">{{ track.title }}</span>
      <span v-if="track.is_video" class="m3-label-small video">Video</span>
    </template>
    <template #trailing>
      <StandingAction :standing="track.standing" @grab="emit('grab')" />
    </template>
  </M3ListItem>
</template>

<script lang="ts" setup>
import type { HuntTrack } from '@/services/huntingService'
import { secondsToHis } from '@/utils/formatters'

import M3ListItem from '@/components/m3/M3ListItem.vue'
import StandingAction from '@/components/screens/hunting/StandingAction.vue'

defineProps<{ track: HuntTrack }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>

<style scoped>
.cover {
  display: block;
  width: 56px;
  height: 56px;
  border-radius: 8px;
  background-color: var(--schemes-surface-container-highest);
  background-size: cover;
  background-position: center;
}

.video {
  margin-left: 6px;
  padding: 2px 6px;
  border-radius: 6px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);
}
</style>
