<template>
  <M3Card class="flex flex-col gap-2.5 p-2">
    <div :style="album.cover ? { backgroundImage: `url(${album.cover})` } : {}" class="cover" />

    <div class="min-w-0 px-1">
      <p :title="album.title" class="m3-title-medium truncate">{{ album.title }}</p>
      <p :title="album.artist" class="m3-body-medium truncate muted">{{ album.artist }}</p>
      <p class="m3-label-medium muted">
        {{ album.kind }}<template v-if="album.year"> · {{ album.year }}</template>
      </p>
    </div>

    <StandingAction
      :library-album="album.library_album"
      :standing="album.standing"
      class="mx-1 mb-1"
      @grab="emit('grab')"
    />
  </M3Card>
</template>

<script lang="ts" setup>
import type { HuntAlbum } from '@/services/huntingService'

import M3Card from '@/components/m3/M3Card.vue'
import StandingAction from '@/components/screens/hunting/StandingAction.vue'

defineProps<{ album: HuntAlbum }>()
const emit = defineEmits<{ (e: 'grab'): void }>()
</script>

<style scoped>
.cover {
  aspect-ratio: 1 / 1;
  border-radius: 12px;
  background-color: var(--schemes-surface-container-highest);
  background-size: cover;
  background-position: center;
}

.muted {
  color: var(--schemes-on-surface-variant);
}
</style>
