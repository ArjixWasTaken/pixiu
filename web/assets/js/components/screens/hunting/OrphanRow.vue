<template>
  <li class="orphan-row" data-vue="OrphanRow">
    <M3Checkbox v-model="selected" :aria-label="`Select ${orphan.song.title}`" :name="`orphan-${orphan.song.id}`" />
    <span :style="orphan.song.album_cover ? { backgroundImage: `url(${orphan.song.album_cover})` } : {}" class="cover">
      <PlatformBadge :platform="orphan.song.source_platform" />
    </span>

    <div class="song flex-1 min-w-0">
      <p :title="orphan.song.title" class="m3-body-large truncate">{{ orphan.song.title }}</p>
      <p class="m3-body-medium truncate muted">{{ orphan.song.artist_name }} · {{ orphan.song.album_name }}</p>
    </div>

    <!-- Beside the song; on a phone, under it, so the song keeps its room. -->
    <div class="why">
      <p :title="orphan.reason" class="m3-body-medium truncate">{{ orphan.reason }}</p>
      <p class="m3-body-small muted">
        <template v-if="orphan.released_at">{{ timeAgo(orphan.released_at) }} · </template
        >{{ formatBytes(orphan.size) }}
      </p>
    </div>
  </li>
</template>

<script lang="ts" setup>
import type { Orphan } from '@/services/huntingService'
import { formatBytes, timeAgo } from '@/utils/formatters'

import M3Checkbox from '@/components/m3/M3Checkbox.vue'
import PlatformBadge from '@/components/ui/PlatformBadge.vue'

defineProps<{ orphan: Orphan }>()
const selected = defineModel<boolean>({ default: false })
</script>

<style scoped>
.orphan-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 0;
  border-top: 1px solid var(--schemes-outline-variant);
  list-style: none;
}

.cover {
  position: relative;
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 8px;
  background-color: var(--schemes-surface-container-highest);
  background-size: cover;
  background-position: center;
}

.muted {
  color: var(--schemes-on-surface-variant);
}

.why {
  flex-shrink: 0;
  max-width: 40%;
  text-align: right;
}

@media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
  .orphan-row {
    flex-wrap: wrap;
    row-gap: 4px;
  }

  .song {
    flex-basis: calc(100% - 120px);
  }

  .why {
    /* Under the title: past the checkbox (40px), the cover (48px) and their gaps. */
    max-width: none;
    width: calc(100% - 120px);
    margin-left: 120px;
    text-align: left;
  }
}
</style>
