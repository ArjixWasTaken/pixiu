<template>
  <M3Card class="flex flex-col gap-3 p-4 mt-3 mb-2" variant="outlined">
    <p class="m3-body-medium">
      Mirrors the watched YouTube Music {{ mirror.watch.kind === 'liked_music' ? 'liked music' : 'playlist' }}
      <a :href="mirror.watch.link" class="text-(--schemes-primary)" rel="noopener" target="_blank"
        >“{{ mirror.watch.name }}”</a
      >, read-only.
      <template v-if="mirror.watch.last_synced_at">Synced {{ timeAgo(mirror.watch.last_synced_at) }}.</template>
      Exclude songs you don’t want from their context menu.
    </p>

    <div v-if="mirror.coming.length || mirror.excluded.length" class="flex flex-wrap gap-2">
      <M3Chip
        v-if="mirror.coming.length"
        :selected="shown === 'coming'"
        variant="filter"
        @click.prevent="toggle('coming')"
      >
        {{ pluralize(mirror.coming, 'song') }} still coming
      </M3Chip>
      <M3Chip
        v-if="mirror.excluded.length"
        :selected="shown === 'excluded'"
        variant="filter"
        @click.prevent="toggle('excluded')"
      >
        {{ pluralize(mirror.excluded, 'song') }} excluded
      </M3Chip>
    </div>

    <M3List v-if="shown === 'coming'" class="py-0!">
      <M3ListItem
        v-for="song in mirror.coming"
        :key="song.video_id"
        :headline="song.title ?? song.video_id"
        :supporting="song.artist ?? undefined"
      />
    </M3List>

    <M3List v-if="shown === 'excluded'" class="py-0!">
      <M3ListItem
        v-for="song in mirror.excluded"
        :key="song.video_id"
        :headline="song.title ?? song.video_id"
        :supporting="[song.artist, timeAgo(song.excluded_at)].filter(Boolean).join(' · ')"
      >
        <template #trailing>
          <M3Button variant="text" @click.prevent="emit('include', song)">Include again</M3Button>
        </template>
      </M3ListItem>
    </M3List>
  </M3Card>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import type { ExcludedSong, PlaylistWatch } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Chip from '@/components/m3/M3Chip.vue'
import M3List from '@/components/m3/M3List.vue'
import M3ListItem from '@/components/m3/M3ListItem.vue'

defineProps<{ mirror: PlaylistWatch }>()
const emit = defineEmits<{ (e: 'include', song: ExcludedSong): void }>()

const shown = ref<'coming' | 'excluded' | null>(null)
const toggle = (section: 'coming' | 'excluded') => (shown.value = shown.value === section ? null : section)
</script>
