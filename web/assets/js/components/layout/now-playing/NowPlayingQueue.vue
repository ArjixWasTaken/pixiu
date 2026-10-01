<template>
  <div class="now-playing-queue">
    <div class="from">
      <div class="min-w-0">
        <p class="m3-body-small muted">Playing from</p>
        <p class="m3-title-small">Current queue</p>
        <p class="m3-body-small muted">{{ meta }}</p>
      </div>
      <M3IconButton
        :disabled="!playables.length"
        icon="playlist_add"
        label="Save queue as playlist"
        variant="tonal"
        @click="saveAsPlaylist"
      />
    </div>

    <ol>
      <li
        v-for="playable in playables"
        :key="playable.id"
        :class="{ current: playable.id === current?.id }"
        class="row m3-state"
        @contextmenu.prevent="openMenu(playable, $event)"
        @dblclick="play(playable)"
      >
        <PlayableThumbnail :playable @clicked="play(playable)" />
        <div class="flex-1 min-w-0">
          <p class="m3-body-large truncate title">{{ playable.title }}</p>
          <p class="m3-body-medium truncate muted">{{ artistOf(playable) }}</p>
        </div>
        <span class="m3-label-medium muted tabular-nums">{{ secondsToHis(playable.length) }}</span>
      </li>
    </ol>

    <p v-if="!playables.length" class="m3-body-medium muted px-4 py-6">Nothing queued.</p>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { queueStore } from '@/stores/queueStore'
import { playback } from '@/services/playbackManager'
import { defineAsyncComponent } from '@/utils/helpers'
import { pluralize, secondsToHis, secondsToHumanReadable } from '@/utils/formatters'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import PlayableThumbnail from '@/components/playable/PlayableThumbnail.vue'

const PlayableContextMenu = defineAsyncComponent(() => import('@/components/playable/PlayableContextMenu.vue'))
const CreatePlaylistForm = defineAsyncComponent(() => import('@/components/playlist/CreatePlaylistForm.vue'))

const { openContextMenu } = useContextMenu()
const { openModal } = useModal()

const current = computed(() => queueStore.current)

/** The current song and what comes after it. */
const playables = computed(() => {
  const all = queueStore.all
  const index = current.value ? all.findIndex(({ id }) => id === current.value!.id) : 0
  return all.slice(Math.max(0, index))
})

const upNext = computed(() => playables.value.slice(current.value ? 1 : 0))

const meta = computed(() => {
  const seconds = upNext.value.reduce((total, { length }) => total + length, 0)
  // What is left after the current song, not the whole queue.
  return `${pluralize(upNext.value, 'song')} left · ${secondsToHumanReadable(seconds)}`
})

const artistOf = (playable: Playable) => playable.artist_name

const play = (playable: Playable) => playback().play(playable)

const openMenu = (playable: Playable, event: MouseEvent) =>
  openContextMenu<'PLAYABLES'>(PlayableContextMenu, event, { playables: [playable] })

const saveAsPlaylist = () =>
  openModal<'CREATE_PLAYLIST_FORM'>(CreatePlaylistForm, { folder: null, playables: queueStore.all })
</script>

<style scoped>
.from {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 4px 16px 8px;
  color: var(--schemes-on-surface);
}

.muted {
  color: var(--schemes-on-surface-variant);
}

.row {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: 64px;
  padding: 8px 16px;
  border-radius: 16px;
  color: var(--schemes-on-surface);
  cursor: pointer;
  list-style: none;

  &.current .title {
    color: var(--schemes-primary);
  }
}
</style>
