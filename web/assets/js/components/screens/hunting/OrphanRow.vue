<template>
  <li class="orphan-row" data-vue="OrphanRow">
    <M3Checkbox v-model="selected" :aria-label="`Select ${orphan.song.title}`" :name="`orphan-${orphan.song.id}`" />
    <span
      :style="orphan.song.album_cover ? { backgroundImage: `url(${orphan.song.album_cover})` } : {}"
      class="cover"
    />

    <div class="flex-1 min-w-0">
      <p :title="orphan.song.title" class="m3-body-large truncate">{{ orphan.song.title }}</p>
      <p class="m3-body-medium truncate muted">{{ orphan.song.artist_name }} · {{ orphan.song.album_name }}</p>
    </div>

    <div class="text-right shrink-0 max-w-[40%]">
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
</style>
