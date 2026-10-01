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
import { commonStore } from '@/stores/commonStore'
import { queueStore } from '@/stores/queueStore'
import { playableStore } from '@/stores/playableStore'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { playback } from '@/services/playbackManager'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import SongListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const totalSongCount = toRef(commonStore.state, 'song_count')
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
} = usePlayableList(toRef(playableStore.state, 'playables'), { type: 'Songs' }, { filterable: false, sortable: true })

const { PlayableListControls: SongListControls, config } = usePlayableListControls('Songs')
const { go, url } = useRouter()

const loading = ref(false)
const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('all-songs-sort-field', 'title')
const sortOrder = useUserStorage<SortOrder>('all-songs-sort-order', 'asc')

const cursor = ref<string | null>('')
const moreSongsAvailable = computed(() => cursor.value !== null)
const showSkeletons = computed(() => loading.value && songs.value.length === 0)

const fetchSongs = async () => {
  if (!moreSongsAvailable.value || loading.value) {
    return
  }

  loading.value = true

  try {
    cursor.value = await playableStore.paginateSongs({
      sort: sortField.value,
      order: sortOrder.value,
      cursor: cursor.value,
    })
  } catch (error: any) {
    useErrorHandler().handleHttpError(error)
  } finally {
    loading.value = false
  }
}

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

const sort = async (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  cursor.value = ''
  playableStore.state.playables = []
  sortField.value = field
  sortOrder.value = order

  await fetchSongs()
}

onMounted(async () => {
  composableSort(sortField.value, sortOrder.value)
  await fetchSongs()
})
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.collapsed .controls {
  @apply w-auto;
}
</style>
