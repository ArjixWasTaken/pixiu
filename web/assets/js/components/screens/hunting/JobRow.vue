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
        <!-- What went wrong: two lines, the rest when asked for (a phone can't hover for a tooltip). -->
        <template v-else-if="job.error && job.state === 'failed'">
          <p :class="{ clamped: !showingError }" class="m3-body-small error">{{ job.error }}</p>
          <button
            v-if="job.error.length > 80"
            :aria-expanded="showingError"
            class="m3-label-medium details"
            type="button"
            @click="showingError = !showingError"
          >
            {{ showingError ? 'Less' : 'Details' }}
          </button>
        </template>
        <M3ProgressIndicator
          v-if="job.state === 'running' && job.progress !== null"
          :value="job.progress"
          class="mt-1"
        />
      </div>

      <template v-if="job.state === 'failed'">
        <M3IconButton v-if="isMobile" icon="refresh" label="Retry" variant="tonal" @click.prevent="emit('retry')" />
        <M3Button v-else icon="refresh" variant="tonal" @click.prevent="emit('retry')">Retry</M3Button>
      </template>
    </M3Card>
  </li>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import type { HuntJob } from '@/services/huntingService'
import { timeAgo } from '@/utils/formatters'
import { useViewport } from '@/composables/useViewport'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3ProgressIndicator from '@/components/m3/M3ProgressIndicator.vue'

const props = defineProps<{ job: HuntJob }>()
const emit = defineEmits<{ (e: 'retry'): void }>()

const { isMobile } = useViewport()
const showingError = ref(false)

const kindLabel = computed(
  () =>
    ({
      download: 'Download',
      album: 'Album',
      watch_sync: 'Watch sync',
      lookup: 'MusicBrainz lookup',
    })[props.job.kind],
)

const icon = computed(
  () =>
    ({
      download: 'download',
      album: 'album',
      watch_sync: 'sync',
      lookup: 'manage_search',
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

.error {
  color: var(--schemes-error);
  overflow-wrap: anywhere;

  &.clamped {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    overflow: hidden;
  }
}

.details {
  align-self: flex-start;
  color: var(--schemes-primary);
}
</style>
