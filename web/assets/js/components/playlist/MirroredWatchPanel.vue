<template>
  <section class="flex flex-col gap-3 p-4 rounded-lg bg-k-fg-5 text-sm">
    <p>
      Mirrors the watched YouTube Music {{ mirror.watch.kind === 'liked_music' ? 'liked music' : 'playlist' }}
      <a :href="mirror.watch.link" rel="noopener" target="_blank">“{{ mirror.watch.name }}”</a>, read-only.
      <template v-if="mirror.watch.last_synced_at">Synced {{ timeAgo(mirror.watch.last_synced_at) }}.</template>
      Exclude songs you don’t want from their context menu.
    </p>

    <div class="flex flex-wrap gap-2">
      <Btn v-if="mirror.coming.length" size="small" variant="ghost" @click.prevent="toggle('coming')">
        {{ pluralize(mirror.coming, 'song') }} still coming
      </Btn>
      <Btn v-if="mirror.excluded.length" size="small" variant="ghost" @click.prevent="toggle('excluded')">
        {{ pluralize(mirror.excluded, 'song') }} excluded
      </Btn>
    </div>

    <ul v-if="shown === 'coming'" class="divide-y divide-k-fg-5">
      <li v-for="song in mirror.coming" :key="song.video_id" class="py-1.5">
        {{ song.title ?? song.video_id }} <span class="text-k-fg-50">{{ song.artist }}</span>
      </li>
    </ul>

    <ul v-if="shown === 'excluded'" class="divide-y divide-k-fg-5">
      <li v-for="song in mirror.excluded" :key="song.video_id" class="flex items-center gap-3 py-1.5">
        <span class="flex-1 min-w-0 truncate">
          {{ song.title ?? song.video_id }} <span class="text-k-fg-50">{{ song.artist }}</span>
        </span>
        <span class="text-k-fg-50 text-xs">{{ timeAgo(song.excluded_at) }}</span>
        <Btn size="small" variant="ghost" @click.prevent="emit('include', song)">Include again</Btn>
      </li>
    </ul>
  </section>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import type { ExcludedSong, PlaylistWatch } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'

import Btn from '@/components/ui/form/Btn.vue'

defineProps<{ mirror: PlaylistWatch }>()
const emit = defineEmits<{ (e: 'include', song: ExcludedSong): void }>()

const shown = ref<'coming' | 'excluded' | null>(null)
const toggle = (section: 'coming' | 'excluded') => (shown.value = shown.value === section ? null : section)
</script>
