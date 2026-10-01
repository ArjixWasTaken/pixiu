<template>
  <M3Card class="flex flex-col gap-3 p-4 mt-3 mb-2" variant="outlined">
    <p class="m3-body-medium">
      <template v-if="mirror.watch.kind === 'liked_music'">
        Mirrors your
        <a :href="mirror.watch.link" class="text-(--schemes-primary)" rel="noopener" target="_blank">liked music</a>
        on YouTube Music, read-only.
      </template>
      <template v-else>
        Mirrors the YouTube Music playlist
        <a :href="mirror.watch.link" class="text-(--schemes-primary)" rel="noopener" target="_blank"
          >“{{ mirror.watch.name }}”</a
        >, read-only.
      </template>
      <template v-if="mirror.watch.last_synced_at">Synced {{ timeAgo(mirror.watch.last_synced_at) }}.</template>
      Exclude songs you don’t want from their menu.
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
        :supporting="[song.artist, standing(song.job)].filter(Boolean).join(' · ')"
      />
      <li class="px-4 pt-2 m3-body-medium list-none">
        <a :href="url('jobs')" class="text-(--schemes-primary)">See the downloads on Jobs</a>
      </li>
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
import { useRouter } from '@/composables/useRouter'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Chip from '@/components/m3/M3Chip.vue'
import M3List from '@/components/m3/M3List.vue'
import M3ListItem from '@/components/m3/M3ListItem.vue'

defineProps<{ mirror: PlaylistWatch }>()
const emit = defineEmits<{ (e: 'include', song: ExcludedSong): void }>()

const { url } = useRouter()

type Coming = PlaylistWatch['coming'][number]

/** How a coming song's download is doing, for people. */
const standing = (job: Coming['job']) => {
  switch (job?.state) {
    case undefined:
      return 'waiting for the next sync'
    case 'queued':
    case 'paused':
      return 'waiting to download'
    case 'running':
      return 'downloading'
    case 'failed':
      return `failed: ${job.error ?? 'unknown error'}`
    case 'done':
      // Downloaded, yet not in the library: it matched a song you have.
      return 'downloaded; waiting for the next sync'
  }
}

const shown = ref<'coming' | 'excluded' | null>(null)
const toggle = (section: 'coming' | 'excluded') => (shown.value = shown.value === section ? null : section)
</script>
