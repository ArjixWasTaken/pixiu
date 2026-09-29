<template>
  <li class="flex items-center gap-4 py-2">
    <img v-if="track.cover" :src="track.cover" alt="" class="size-10 rounded-sm object-cover" loading="lazy" />
    <div v-else class="size-10 rounded-sm bg-k-fg-10" />

    <div class="flex-1 min-w-0">
      <p :title="track.title" class="truncate">
        {{ track.title }}
        <span v-if="track.is_video" class="text-k-fg-50 text-xs uppercase ml-1">Video</span>
      </p>
      <p class="truncate text-k-fg-70 text-sm">
        {{ track.artist }}<template v-if="track.album"> · {{ track.album }}</template>
      </p>
    </div>

    <span v-if="track.length" class="text-k-fg-50 text-sm tabular-nums">{{ secondsToHis(track.length) }}</span>

    <StandingAction :standing="track.standing" class="w-32" @grab="emit('grab')" />
  </li>
</template>

<script lang="ts" setup>
import type { HuntTrack } from '@/services/huntingService'
import { secondsToHis } from '@/utils/formatters'

import StandingAction from '@/components/screens/hunting/StandingAction.vue'

defineProps<{ track: HuntTrack }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>
