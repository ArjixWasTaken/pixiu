<template>
  <article class="flex flex-col gap-2 p-3 rounded-lg bg-k-fg-5">
    <img
      v-if="album.cover"
      :src="album.cover"
      alt=""
      class="aspect-square w-full rounded-md object-cover"
      loading="lazy"
    />
    <div v-else class="aspect-square w-full rounded-md bg-k-fg-10" />

    <div class="min-w-0">
      <p :title="album.title" class="truncate font-medium">{{ album.title }}</p>
      <p :title="album.artist" class="truncate text-k-fg-70 text-sm">{{ album.artist }}</p>
      <p class="text-k-fg-50 text-xs">
        {{ album.kind }}<template v-if="album.year"> · {{ album.year }}</template>
      </p>
    </div>

    <StandingAction :standing="album.standing" @grab="emit('grab')" />
  </article>
</template>

<script lang="ts" setup>
import type { HuntAlbum } from '@/services/huntingService'

import StandingAction from '@/components/screens/hunting/StandingAction.vue'

defineProps<{ album: HuntAlbum }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>
