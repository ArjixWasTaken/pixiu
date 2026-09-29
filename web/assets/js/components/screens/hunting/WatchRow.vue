<template>
  <li class="flex items-center gap-4 p-4 rounded-lg bg-k-fg-5">
    <img v-if="watch.image" :src="watch.image" alt="" class="size-16 rounded-md object-cover" loading="lazy" />
    <div v-else class="size-16 rounded-md bg-k-fg-10 flex items-center justify-center text-k-fg-50">
      <Icon :icon="kindIcon" size="lg" />
    </div>

    <div class="flex-1 min-w-0 flex flex-col gap-1">
      <p class="truncate font-medium">
        {{ watch.name }}
        <span class="text-k-fg-50 text-xs uppercase ml-1">{{ kindLabel }}</span>
      </p>

      <p class="text-sm text-k-fg-70">
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

      <p v-if="watch.status.error" class="text-sm text-k-danger truncate" :title="watch.status.error">
        {{ watch.status.error }}
      </p>
    </div>

    <div class="flex gap-2 shrink-0">
      <Btn
        v-if="watch.playlist_id"
        :href="url('playlists.show', { id: watch.playlist_id })"
        size="small"
        tag="a"
        variant="ghost"
      >
        Playlist
      </Btn>
      <Btn :href="watch.link" rel="noopener" size="small" tag="a" target="_blank" variant="ghost">
        <Icon :icon="faArrowUpRightFromSquare" />
      </Btn>
      <Btn :disabled="busy" size="small" @click.prevent="emit('sync')">Sync now</Btn>
      <Btn size="small" variant="destructive" @click.prevent="emit('remove')">Remove</Btn>
    </div>
  </li>
</template>

<script lang="ts" setup>
import { faArrowUpRightFromSquare, faHeart, faListUl, faMicrophone } from '@fortawesome/free-solid-svg-icons'
import { computed } from 'vue'
import type { Watch } from '@/services/huntingService'
import { pluralize, timeAgo } from '@/utils/formatters'
import { useRouter } from '@/composables/useRouter'

import Btn from '@/components/ui/form/Btn.vue'

const props = defineProps<{ watch: Watch }>()
const emit = defineEmits<{ (e: 'sync'): void; (e: 'remove'): void }>()

const { url } = useRouter()

const kindLabel = computed(
  () => ({ playlist: 'Playlist', liked_music: 'Liked music', artist: 'Artist' })[props.watch.kind],
)

const kindIcon = computed(() => ({ playlist: faListUl, liked_music: faHeart, artist: faMicrophone })[props.watch.kind])

const busy = computed(() => ['syncing', 'queued'].includes(props.watch.status.state))

const statusLabel = computed(() => {
  switch (props.watch.status.state) {
    case 'syncing':
      return 'Syncing…'
    case 'queued':
      return 'Sync queued'
    case 'waiting':
      return 'Waiting for a YouTube Music login'
    case 'failed':
      return 'Last sync failed'
    case 'never_synced':
      return 'Not synced yet'
    default:
      return `Synced ${timeAgo(props.watch.last_synced_at)}`
  }
})
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';

.status {
  &.syncing,
  &.queued {
    @apply text-k-highlight;
  }

  &.waiting {
    @apply text-k-warning;
  }

  &.failed {
    @apply text-k-danger;
  }
}

.failed {
  @apply text-k-danger;
}
</style>
