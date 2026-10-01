<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader :disabled="loading" :layout="songs.length ? headerLayout : 'collapsed'">
        All songs

        <template #thumbnail>
          <ThumbnailStack :thumbnails />
        </template>

        <template v-if="totalSongCount" #meta>
          <span>{{ pluralize(totalSongCount, 'song') }}</span>
          <span>{{ totalDuration }}</span>
        </template>

        <template #controls>
          <div class="controls w-full flex justify-between items-center gap-4">
            <SongListControls v-if="totalSongCount" :config @play-all="playAll" @play-selected="playSelected" />
          </div>
        </template>
      </ScreenHeader>
    </template>

    <SongListSkeleton v-if="showSkeletons" class="screen-bleed" role="status" aria-busy="true" aria-label="Loading" />
    <template v-else>
      <SongList
        v-if="songs?.length > 0"
        ref="songList"
        class="screen-bleed"
        @sort="sort"
        @swipe="onSwipe"
        @press:enter="onPressEnter"
        @scrolled-to-end="fetchSongs"
      />
      <ScreenEmptyState v-else>
        <template #icon>
          <M3Icon name="volume_off" />
        </template>
        Your library is empty.
      </ScreenEmptyState>
    </template>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, toRef } from 'vue'
import { pluralize, secondsToHumanReadable } from '@/utils/formatters'
import { useCommonStore } from '@/stores/commonStore'
import { useQueueStore } from '@/stores/queueStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useRouter } from '@/composables/useRouter'
import { useListPages } from '@/composables/useListPages'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { playback } from '@/services/playbackManager'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import SongListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const commonStore = useCommonStore()
const queueStore = useQueueStore()
const playableStore = usePlayableStore()

const totalSongCount = toRef(commonStore.state, 'song_count')

const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('all-songs-sort-field', 'title')
const sortOrder = useUserStorage<SortOrder>('all-songs-sort-order', 'asc')

// Each sort is a list of its own; changing it starts the list from its first page.
const {
  items: allSongs,
  isFetching: loading,
  fetchMore: fetchSongs,
} = useListPages(
  () => ['songs', { sort: sortField.value, order: sortOrder.value }],
  cursor => playableStore.paginateSongs({ sort: sortField.value, order: sortOrder.value, cursor }),
  // Re-sorting keeps the list (and the focus in its header) until the new order comes.
  { keepPrevious: true },
)
const totalDuration = computed(() => secondsToHumanReadable(commonStore.state.song_length))

const {
  PlayableList: SongList,
  ThumbnailStack,
  headerLayout,
  thumbnails,
  playables: songs,
  playableList: songList,
  onPressEnter,
  playSelected,
  onSwipe,
  sort: composableSort,
} = usePlayableList(allSongs, { type: 'Songs' }, { filterable: false, sortable: true })

const { PlayableListControls: SongListControls, config } = usePlayableListControls('Songs')
const { go, url } = useRouter()

const showSkeletons = computed(() => loading.value && songs.value.length === 0)

const playAll = async (shuffle: boolean) => {
  if (shuffle) {
    await queueStore.fetchRandom()
  } else {
    await queueStore.fetchInOrder(
      Array.isArray(sortField.value) ? sortField.value[0] : sortField.value,
      sortOrder.value,
    )
  }

  go(url('queue'))
  await playback().playFirstInQueue()
}

const sort = (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  sortField.value = field
  sortOrder.value = order
}

onMounted(() => composableSort(sortField.value, sortOrder.value))
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.collapsed .controls {
  @apply w-auto;
}
</style>
