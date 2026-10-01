<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader :layout="playables.length === 0 ? 'collapsed' : headerLayout">
        Available offline

        <template #thumbnail>
          <ThumbnailStack :thumbnails="thumbnails" />
        </template>

        <template v-if="playables.length" #meta>
          <span>{{ pluralize(playables, 'song') }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <PlayableListControls v-if="playables.length" :config @play-all="playAll" @play-selected="playSelected" />
        </template>
      </ScreenHeader>
    </template>

    <PlayableList
      v-if="playables.length"
      ref="playableList"
      class="screen-bleed"
      @press:enter="onPressEnter"
      @swipe="onSwipe"
    />

    <ScreenEmptyState v-else>
      <template #icon>
        <M3Icon name="cloud_download" />
      </template>
      No songs available offline.
      <span class="secondary block"> Right-click a song and choose “Make available offline” to keep a copy. </span>
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { pluralize } from '@/utils/formatters'
import { usePlayableStore } from '@/stores/playableStore'
import { useOfflinePlayback } from '@/composables/useOfflinePlayback'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const playableStore = usePlayableStore()

const { cachedSongIds } = useOfflinePlayback()

const offlineSongs = computed(() => {
  const songs: Playable[] = []

  for (const id of cachedSongIds.value) {
    const song = playableStore.byId(id)
    if (song) songs.push(song)
  }

  return songs
})

const {
  PlayableList,
  ThumbnailStack,
  headerLayout,
  playables,
  playableList,
  thumbnails,
  duration,
  onPressEnter,
  playAll,
  playSelected,
  onSwipe,
} = usePlayableList(offlineSongs, { type: 'OfflineSongs' }, { sortable: true })

const { PlayableListControls, config } = usePlayableListControls('OfflineSongs')
</script>
