<template>
  <li class="flex items-center gap-4 py-2">
    <CheckBox v-model="selected" :name="`orphan-${orphan.song.id}`" />
    <img
      v-if="orphan.song.album_cover"
      :src="orphan.song.album_cover"
      alt=""
      class="size-10 rounded-sm object-cover"
      loading="lazy"
    />
    <div v-else class="size-10 rounded-sm bg-k-fg-10" />

    <div class="flex-1 min-w-0">
      <p :title="orphan.song.title" class="truncate">{{ orphan.song.title }}</p>
      <p class="truncate text-sm text-k-fg-70">{{ orphan.song.artist_name }} · {{ orphan.song.album_name }}</p>
    </div>

    <div class="text-right text-sm shrink-0 max-w-[40%]">
      <p :title="orphan.reason" class="truncate">{{ orphan.reason }}</p>
      <p class="text-k-fg-50">
        <template v-if="orphan.released_at">{{ timeAgo(orphan.released_at) }} · </template
        >{{ formatBytes(orphan.size) }}
      </p>
    </div>
  </li>
</template>

<script lang="ts" setup>
import type { Orphan } from '@/services/huntingService'
import { formatBytes, timeAgo } from '@/utils/formatters'

import CheckBox from '@/components/ui/form/CheckBox.vue'

defineProps<{ orphan: Orphan }>()
const selected = defineModel<boolean>({ default: false })
</script>
