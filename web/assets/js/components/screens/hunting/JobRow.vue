<template>
  <li class="list-none">
    <M3Card class="flex items-center gap-4 px-4 py-3">
      <span class="kind-icon">
        <M3Icon :name="icon" :size="20" />
      </span>

      <div class="flex-1 min-w-0 flex flex-col gap-1">
        <p :title="job.title" class="m3-body-large truncate">{{ job.title }}</p>
        <p class="m3-body-small muted">
          {{ kindLabel }}
          <template v-if="job.family"> · {{ job.family.done }} of {{ job.family.total }} songs</template>
          <template v-if="job.family?.failed"> · {{ job.family.failed }} failed</template>
          · {{ job.state === 'done' ? 'finished' : 'queued' }} {{ timeAgo(job.finished_at ?? job.created_at) }}
        </p>
        <p v-if="job.state === 'paused'" class="m3-body-small text-(--schemes-tertiary)">
          {{ job.error ?? 'Paused until you log in to YouTube Music' }}
        </p>
        <p
          v-else-if="job.error && job.state === 'failed'"
          :title="job.error"
          class="m3-body-small truncate text-(--schemes-error)"
        >
          {{ job.error }}
        </p>
        <M3ProgressIndicator
          v-if="job.state === 'running' && job.progress !== null"
          :value="job.progress"
          class="mt-1"
        />
      </div>

      <M3Button v-if="job.state === 'failed'" icon="refresh" variant="tonal" @click.prevent="emit('retry')"
        >Retry</M3Button
      >
    </M3Card>
  </li>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import type { HuntJob } from '@/services/huntingService'
import { timeAgo } from '@/utils/formatters'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

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
      download: 'download',
      album: 'album',
      watch_sync: 'sync',
      lookup: 'manage_search',
      refile: 'drive_file_move',
    })[props.job.kind],
)
</script>

<style scoped>
.kind-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  border-radius: 20px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);
}

.muted {
  color: var(--schemes-on-surface-variant);
}
</style>
