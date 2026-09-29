<template>
  <li class="flex items-center gap-4 px-4 py-3 rounded-lg bg-k-fg-5">
    <Icon :icon="icon" class="text-k-fg-50" fixed-width />

    <div class="flex-1 min-w-0 flex flex-col gap-1">
      <p :title="job.title" class="truncate">{{ job.title }}</p>
      <p class="text-xs text-k-fg-50">
        {{ kindLabel }}
        <template v-if="job.family"> · {{ job.family.done }} of {{ job.family.total }} songs</template>
        <template v-if="job.family?.failed"> · {{ job.family.failed }} failed</template>
        · {{ job.state === 'done' ? 'finished' : 'queued' }} {{ timeAgo(job.finished_at ?? job.created_at) }}
      </p>
      <p v-if="job.state === 'paused'" class="text-xs text-k-warning">
        {{ job.error ?? 'Paused until you log in to YouTube Music' }}
      </p>
      <p v-else-if="job.error && job.state === 'failed'" :title="job.error" class="text-xs text-k-danger truncate">
        {{ job.error }}
      </p>
      <div v-if="job.state === 'running' && job.progress !== null" class="h-1 rounded-full bg-k-fg-10 overflow-hidden">
        <div
          :style="{ width: `${Math.round(job.progress * 100)}%` }"
          class="h-full bg-k-highlight transition-[width]"
        />
      </div>
    </div>

    <Btn v-if="job.state === 'failed'" size="small" @click.prevent="emit('retry')">Retry</Btn>
  </li>
</template>

<script lang="ts" setup>
import { faCompactDisc, faDownload, faFolderTree, faMagnifyingGlass, faRotate } from '@fortawesome/free-solid-svg-icons'
import { computed } from 'vue'
import type { HuntJob } from '@/services/huntingService'
import { timeAgo } from '@/utils/formatters'

import Btn from '@/components/ui/form/Btn.vue'

const props = defineProps<{ job: HuntJob }>()
const emit = defineEmits<{ (e: 'retry'): void }>()

const kindLabel = computed(
  () =>
    ({
      download: 'Download',
      album: 'Album',
      watch_sync: 'Watch sync',
      lookup: 'MusicBrainz lookup',
      refile: 'Moving files',
    })[props.job.kind],
)

const icon = computed(
  () =>
    ({
      download: faDownload,
      album: faCompactDisc,
      watch_sync: faRotate,
      lookup: faMagnifyingGlass,
      refile: faFolderTree,
    })[props.job.kind],
)
</script>
