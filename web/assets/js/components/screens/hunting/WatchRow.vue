<template>
  <li class="list-none">
    <M3Card class="flex items-center gap-4 flex-wrap p-4" variant="outlined">
      <div :style="watch.image ? { backgroundImage: `url(${watch.image})` } : {}" class="art">
        <M3Icon v-if="!watch.image" :name="kindIcon" fill />
        <PlatformBadge :platform="watch.platform" />
      </div>

      <div class="flex-1 min-w-[220px] flex flex-col gap-1">
        <div class="flex items-center gap-2 min-w-0">
          <p class="m3-title-medium truncate">{{ watch.name }}</p>
          <span class="m3-label-small kind">{{ kindLabel }}</span>
        </div>
        <p class="m3-body-medium muted">
          <span :class="watch.status.state" class="status">{{ statusLabel }}</span>
          <template v-if="watch.songs">
            · {{ watch.songs.have }} of {{ watch.songs.total }} songs in your library</template
          >
          <template v-if="watch.kind === 'artist'">
            · {{ pluralize(watch.releases_known, 'release') }} known ·
            {{ watch.include_singles ? 'albums, singles and EPs' : 'albums only' }}
          </template>
          <template v-if="watch.jobs.queued"> · {{ watch.jobs.queued }} on the way</template>
          <a v-if="watch.jobs.failed" :href="url('jobs')" class="failed"> · {{ watch.jobs.failed }} failed</a>
        </p>
        <p v-if="watch.status.error" :title="watch.status.error" class="m3-body-small failed truncate">
          {{ watch.status.error }}
        </p>
      </div>

      <div class="flex gap-1 items-center flex-wrap">
        <M3Button v-if="watch.playlist_id" :href="url('playlists.show', { id: watch.playlist_id })" variant="text">
          Playlist
        </M3Button>
        <a :href="watch.link" rel="noopener" target="_blank">
          <M3IconButton :label="`Open on ${platform}`" icon="open_in_new" />
        </a>
        <M3Button :disabled="busy" variant="tonal" @click.prevent="emit('sync')">Sync now</M3Button>
        <M3Button class="remove" variant="text" @click.prevent="emit('remove')">Remove</M3Button>
      </div>
    </M3Card>
  </li>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import type { Watch } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'
import { useRouter } from '@/composables/useRouter'
import { platformName } from '@/config/platforms'

import M3Button from '@/components/m3/M3Button.vue'
import M3Card from '@/components/m3/M3Card.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlatformBadge from '@/components/ui/PlatformBadge.vue'

const props = defineProps<{ watch: Watch }>()
const emit = defineEmits<{ (e: 'sync'): void; (e: 'remove'): void }>()

const { url } = useRouter()

const kindLabel = computed(
  () => ({ playlist: 'Playlist', liked_music: 'Liked music', artist: 'Artist' })[props.watch.kind],
)

const kindIcon = computed(
  () => ({ playlist: 'queue_music', liked_music: 'favorite', artist: 'artist' })[props.watch.kind],
)

const platform = computed(() => platformName(props.watch.platform))

const busy = computed(() => ['syncing', 'queued'].includes(props.watch.status.state))

const statusLabel = computed(() => {
  switch (props.watch.status.state) {
    case 'syncing':
      return 'Syncing…'
    case 'queued':
      return 'Sync queued'
    case 'waiting':
      return `Waiting for you to sign in to ${platform.value}`
    case 'failed':
      return 'Last sync failed'
    case 'never_synced':
      return 'Not synced yet'
    default:
      return `Synced ${timeAgo(props.watch.last_synced_at)}`
  }
})
</script>

<style scoped>
.art {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 64px;
  height: 64px;
  flex-shrink: 0;
  border-radius: 12px;
  background-color: var(--schemes-secondary-container);
  background-size: cover;
  background-position: center;
  color: var(--schemes-on-secondary-container);
}

.kind {
  flex-shrink: 0;
  padding: 2px 8px;
  border-radius: 8px;
  background: var(--schemes-secondary-container);
  color: var(--schemes-on-secondary-container);
}

.muted {
  color: var(--schemes-on-surface-variant);
}

.status {
  &.syncing,
  &.queued {
    color: var(--schemes-primary);
  }

  &.waiting {
    color: var(--schemes-tertiary);
  }

  &.failed {
    color: var(--schemes-error);
  }
}

.failed,
.remove {
  color: var(--schemes-error);
}
</style>
