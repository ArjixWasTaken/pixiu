<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader v-if="genre" :layout="headerLayout">
        <template v-if="genre.name">
          <span class="font-thin">Genre:</span>
          {{ genre.name }}
        </template>
        <span v-else class="font-thin italic">No genre</span>

        <template #thumbnail>
          <ThumbnailStack :thumbnails />
        </template>

        <template v-if="genre" #meta>
          <span>{{ pluralize(genre.song_count, 'song') }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <SongListControls :config @play-all="playAll" @play-selected="playSelected">
            <M3IconButton icon="more_vert" label="More actions" @click="requestContextMenu" />
          </SongListControls>
        </template>
      </ScreenHeader>
      <ScreenHeaderSkeleton v-else role="status" aria-busy="true" aria-label="Loading" />
    </template>

    <PlayableListSkeleton
      v-if="showSkeletons"
      class="screen-bleed"
      role="status"
      aria-busy="true"
      aria-label="Loading"
    />
    <SongList
      v-else
      ref="songList"
      class="screen-bleed"
      @sort="fetchWithSort"
      @press:enter="onPressEnter"
      @swipe="onSwipe"
      @scrolled-to-end="fetch"
    />

    <LoadFailedState v-if="loadFailed" what="songs" @retry="refetch" />
    <ScreenEmptyState v-else-if="!songs.length && !loading">
      <template #icon>
        <M3Icon name="category" :size="96" />
      </template>

      No songs in this genre.
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { useQuery } from '@tanstack/vue-query'
import { computed, onMounted, ref, watch } from 'vue'
import { pluralize, secondsToHumanReadable } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { defineAsyncComponent } from '@/utils/helpers'
import { useGenreStore } from '@/stores/genreStore'
import { usePlayableStore } from '@/stores/playableStore'
import { playback } from '@/services/playbackManager'
import { queryClient } from '@/services/queryClient'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { useListPages } from '@/composables/useListPages'
import { useContextMenu } from '@/composables/useContextMenu'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import LoadFailedState from '@/components/ui/LoadFailedState.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenHeaderSkeleton from '@/components/ui/ScreenHeaderSkeleton.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const genreStore = useGenreStore()
const playableStore = usePlayableStore()

const ContextMenu = defineAsyncComponent(() => import('@/components/genre/GenreContextMenu.vue'))

const { getRouteParam, isCurrentScreen, go, onRouteChanged, url } = useRouter()

const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('genre-sort-field', 'title')
const sortOrder = useUserStorage<SortOrder>('genre-sort-order', 'asc')

const id = ref<Genre['id'] | null>(null)

const { data: genre, error } = useQuery({
  queryKey: computed(() => ['genre', id.value]),
  queryFn: () => genreStore.fetchOne(id.value!),
  enabled: computed(() => Boolean(id.value)),
})
watch(error, error => error && useErrorHandler('dialog').handleHttpError(error))

// Each sort is a list of its own; changing it starts the list from its first page.
const {
  items: songs,
  isFetching: loading,
  fetchMore: fetch,
  loadFailed,
  refetch,
} = useListPages(
  () => ['genre', id.value, 'songs', { sort: sortField.value, order: sortOrder.value }],
  cursor => playableStore.paginateSongsByGenre(id.value!, { sort: sortField.value, order: sortOrder.value, cursor }),
  { enabled: () => Boolean(id.value) },
)

const {
  PlayableList: SongList,
  ThumbnailStack,
  headerLayout,
  playableList: songList,
  thumbnails,
  onPressEnter,
  playSelected,
  onSwipe,
  sort: composableSort,
} = usePlayableList(songs, { type: 'Genre' }, { sortable: true, filterable: false })

const { PlayableListControls: SongListControls, config } = usePlayableListControls('Genre')
const { openContextMenu } = useContextMenu()

const showSkeletons = computed(() => loading.value && songs.value.length === 0)
const duration = computed(() => (genre.value ? secondsToHumanReadable(genre.value.length) : ''))

const fetchWithSort = (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  sortField.value = field
  sortOrder.value = order
}

const getIdFromRoute = () => getRouteParam('id') ?? null

onRouteChanged(route => {
  if (route.screen === 'Genre') {
    id.value = getIdFromRoute()
  }
})

const playAll = async (shuffle = false) => {
  if (!genre.value) {
    return
  }

  go(url('queue'))

  if (shuffle) {
    await playback().queueAndPlay(await playableStore.fetchSongsByGenre(genre.value!, true))
  } else {
    await playback().queueAndPlay(await playableStore.fetchSongsByGenre(genre.value!, false))
  }
}

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'GENRE'>(ContextMenu, event, {
    genre: genre.value!,
  })

onMounted(() => {
  composableSort(sortField.value, sortOrder.value)

  if (isCurrentScreen('Genre')) {
    id.value = getIdFromRoute()
  }
})

// Which genres an edit touched isn't known: all of them are fetched again.
eventBus.on('SONGS_UPDATED', () => queryClient.invalidateQueries({ queryKey: ['genre'] }))
</script>
