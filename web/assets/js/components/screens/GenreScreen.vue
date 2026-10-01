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

    <ScreenEmptyState v-if="!songs.length && !loading">
      <template #icon>
        <M3Icon name="category" :size="96" />
      </template>

      No songs in this genre.
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref, watch } from 'vue'
import { pluralize, secondsToHumanReadable } from '@/utils/formatters'
import { eventBus } from '@/utils/eventBus'
import { defineAsyncComponent } from '@/utils/helpers'
import { genreStore } from '@/stores/genreStore'
import { playableStore } from '@/stores/playableStore'
import { playback } from '@/services/playbackManager'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { useContextMenu } from '@/composables/useContextMenu'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenHeaderSkeleton from '@/components/ui/ScreenHeaderSkeleton.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const ContextMenu = defineAsyncComponent(() => import('@/components/genre/GenreContextMenu.vue'))

const songs = ref<Song[]>([])

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
const { getRouteParam, isCurrentScreen, go, onRouteChanged, url } = useRouter()
const { openContextMenu } = useContextMenu()

const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('genre-sort-field', 'title')
const sortOrder = useUserStorage<SortOrder>('genre-sort-order', 'asc')

const id = ref<Genre['id'] | null>(null)
const genre = ref<Genre | null>(null)
const loading = ref(false)
const cursor = ref<string | null>('')

const moreSongsAvailable = computed(() => cursor.value !== null)
const showSkeletons = computed(() => loading.value && songs.value.length === 0)
const duration = computed(() => (genre.value ? secondsToHumanReadable(genre.value.length) : ''))

const fetch = async () => {
  if (!moreSongsAvailable.value || loading.value) {
    return
  }

  loading.value = true

  try {
    let fetched: { songs: Song[]; nextCursor: string | null }

    ;[genre.value, fetched] = await Promise.all([
      genreStore.fetchOne(id.value!),
      playableStore.paginateSongsByGenre(id.value!, {
        sort: sortField.value,
        order: sortOrder.value,
        cursor: cursor.value,
      }),
    ])

    cursor.value = fetched.nextCursor
    songs.value.push(...fetched.songs)
  } catch (error: unknown) {
    useErrorHandler('dialog').handleHttpError(error)
  } finally {
    loading.value = false
  }
}

const refresh = async () => {
  genre.value = null
  cursor.value = ''
  songs.value = []

  await fetch()
}

const fetchWithSort = async (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  cursor.value = ''
  songs.value = []
  sortField.value = field
  sortOrder.value = order

  await fetch()
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

watch(id, async () => id.value && (await refresh()))

// We can't really tell how/if the genres have been updated, so we just refresh the list
eventBus.on('SONGS_UPDATED', async () => genre.value && (await refresh()))
</script>
