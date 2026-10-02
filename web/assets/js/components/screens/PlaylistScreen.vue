<template>
  <ScreenBase v-if="playlistId" :tint-from="playlist?.cover || thumbnails[0]">
    <template #header>
      <ScreenHeader
        v-if="playlist"
        :disabled="loading"
        :layout="allPlayables.length ? headerLayout : 'collapsed'"
        :overline="playlist.is_smart ? 'Smart playlist' : 'Playlist'"
      >
        {{ playlist.name }}

        <template v-if="playlist.description" #description>{{ playlist.description }}</template>

        <template #thumbnail>
          <PlaylistThumbnail :playlist>
            <ThumbnailStack v-if="!playlist.cover" :thumbnails />
          </PlaylistThumbnail>
        </template>

        <template v-if="filteredPlayables.length" #meta>
          <span>{{ songCount }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <PlayableListControls
            :config="controlsConfig"
            @refresh="fetchDetails(true)"
            @play-all="playAll"
            @play-selected="playSelected"
          >
            <M3IconButton icon="more_vert" label="More actions" @click="requestContextMenu" />
          </PlayableListControls>
        </template>
      </ScreenHeader>
      <ScreenHeaderSkeleton v-else role="status" aria-busy="true" aria-label="Loading" />
    </template>

    <PlayableListSkeleton v-if="loading" class="screen-bleed" role="status" aria-busy="true" aria-label="Loading" />
    <template v-else>
      <MirroredWatchPanel v-if="mirror" :mirror @include="includeAgain" />

      <PlayableList
        v-if="filteredPlayables.length"
        ref="playableList"
        class="screen-bleed"
        @reorder="onReorder"
        @sort="sort"
        @press:delete="removeSelected"
        @press:enter="onPressEnter"
        @swipe="onSwipe"
      />

      <ScreenEmptyState v-else>
        <template #icon>
          <M3Icon name="description" />
        </template>

        <template v-if="playlist?.is_smart">
          <p>
            No songs match the playlist's
            <a class="inline" @click.prevent="editPlaylist">criteria</a>.
          </p>
        </template>
        <template v-else>
          The playlist is empty.
          <span class="block secondary">
            Add songs with “Add to” in a song’s menu{{
              isTouch ? '' : ', or drag them onto the playlist in the sidebar'
            }}.
          </span>
        </template>
      </ScreenEmptyState>
    </template>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { differenceBy } from 'lodash-es'
import { computed, ref, watch } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { logger } from '@/utils/logger'
import type { ExcludedSong } from '@/services/huntingService'
import { useHuntingStore } from '@/stores/huntingStore'
import { useMessageToaster } from '@/composables/useMessageToaster'
import { usePlaylistStore } from '@/stores/playlistStore'
import { usePlayableStore } from '@/stores/playableStore'
import { defineAsyncComponent } from '@/utils/helpers'
import { useRouter } from '@/composables/useRouter'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlaylistContentManagement } from '@/composables/usePlaylistContentManagement'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { useContextMenu } from '@/composables/useContextMenu'
import { useModal } from '@/composables/useModal'
import { useViewport } from '@/composables/useViewport'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import PlaylistThumbnail from '@/components/ui/PlaylistThumbnail.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ScreenHeaderSkeleton from '@/components/ui/ScreenHeaderSkeleton.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import MirroredWatchPanel from '@/components/playlist/MirroredWatchPanel.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const { isTouch } = useViewport()
const huntingStore = useHuntingStore()
const playlistStore = usePlaylistStore()
const playableStore = usePlayableStore()

const ContextMenu = defineAsyncComponent(() => import('@/components/playlist/PlaylistContextMenu.vue'))
const EditPlaylistForm = defineAsyncComponent(() => import('@/components/playlist/EditPlaylistForm.vue'))
const EditSmartPlaylistForm = defineAsyncComponent(
  () => import('@/components/playlist/smart-playlist/EditSmartPlaylistForm.vue'),
)

// Since this component is responsible for all playlists, we keep track of the state for each,
// so that filter and sort settings are preserved when switching between them.
// Playlists and content are (re)fetched (and cached) by the stores on demand, so we don't need to keep them in state.
interface PlaylistScreenState {
  filterKeywords: string
  sortField: MaybeArray<PlayableListSortField> | null
  sortOrder: SortOrder | null
}

const { triggerNotFound, getRouteParam, onScreenActivated, go, url } = useRouter()
const { openContextMenu } = useContextMenu()
const { openModal } = useModal()
/** Each playlist's own sort, by its id. */
const playlistSorts = useUserStorage<
  Record<Playlist['id'], { field: MaybeArray<PlayableListSortField> | null; order: SortOrder }>
>('playlist-sorts', {})

const states = new Map<Playlist['id'], PlaylistScreenState>()

const blankState = (id?: Playlist['id']): PlaylistScreenState => {
  return {
    filterKeywords: '',
    sortField: (id && playlistSorts.value[id]?.field) || null,
    sortOrder: (id && playlistSorts.value[id]?.order) || 'asc',
  }
}

const getState = (id: Playlist['id']) => {
  if (!states.has(id)) {
    states.set(id, blankState(id))
  }

  return states.get(id)!
}

let currentState = blankState()
const allPlayables = ref<Playable[]>([])

const playlistId = ref<Playlist['id']>()
const playlist = ref<Playlist>()
const loading = ref(false)

const {
  PlayableList,
  ThumbnailStack,
  headerLayout,
  playables: filteredPlayables,
  playableList,
  duration,
  thumbnails,
  selectedPlayables,
  context,
  filterKeywords,
  onPressEnter,
  playAll,
  playSelected,
  onSwipe,
  sort: baseSort,
  config: listConfig,
  songCount,
} = usePlayableList(allPlayables, { type: 'Playlist' })

const { PlayableListControls, config: controlsConfig } = usePlayableListControls('Playlist')
const { removeFromPlaylist } = usePlaylistContentManagement()

watch(filterKeywords, keywords => {
  // keep track of the keywords in the state
  currentState.filterKeywords = keywords
})

const sort = (field: MaybeArray<PlayableListSortField> | null, order: SortOrder) => {
  listConfig.reorderable = field === 'position'

  currentState.sortField = field
  currentState.sortOrder = order

  if (playlistId.value) {
    playlistSorts.value = { ...playlistSorts.value, [playlistId.value]: { field, order } }
  }

  // We always call the base sort function, which will handle the actual sorting logic.
  // For the 'position' field, which actually doesn't use the base sort function, we call it anyway
  // to properly keep track of sortField and sortOrder in useSongList, ensuring the UI reflects these correctly.
  baseSort(field, order)

  if (field === 'position') {
    // To sort by position, we simply re-assign the playable array from the playlist, which maintains the original order.
    allPlayables.value = playlist.value!.playables!
  }
}

const editPlaylist = () => {
  const p = playlist.value!
  p.is_smart
    ? openModal<'EDIT_SMART_PLAYLIST_FORM'>(EditSmartPlaylistForm, { playlist: p })
    : openModal<'EDIT_PLAYLIST_FORM'>(EditPlaylistForm, { playlist: p })
}

const removeSelected = async () => {
  // Mirrors of watched playlists change on YouTube Music only.
  if (playlist.value?.permissions.edit) {
    await removeFromPlaylist(playlist.value, selectedPlayables.value)
  }
}

/** For a mirror of a watched playlist, the watch and its exclusions. */
const mirror = computed(() => (playlist.value ? (huntingStore.playlistWatches[playlist.value.id] ?? null) : null))

const fetchMirror = () => playlist.value && huntingStore.fetchPlaylistWatch(playlist.value).catch(logger.error)

const includeAgain = async (song: ExcludedSong) => {
  try {
    await huntingStore.include(mirror.value!.watch.id, song.video_id)
    toastSuccess(`“${song.title ?? song.video_id}” is back on the watch. A sync is on its way.`)
  } catch (error: unknown) {
    useErrorHandler().handleHttpError(error)
  }
}

const fetchDetails = async (refresh = false) => {
  const shown = playlist.value!

  try {
    loading.value = true
    const songs = await playableStore.fetchForPlaylist(shown, refresh)

    // Another playlist may have opened meanwhile: it shows its own songs, once they come.
    if (playlist.value === shown) {
      allPlayables.value = songs
    }
  } catch (error: unknown) {
    useErrorHandler().handleHttpError(error)
  } finally {
    if (playlist.value === shown) {
      loading.value = false
    }
  }
}

const onReorder = (target: Playable, placement: Placement) => {
  playlistStore.moveItemsInPlaylist(playlist.value!, selectedPlayables.value, target, placement)
}

watch(playlistId, async id => {
  if (!id) {
    return
  }

  playlist.value = playlistStore.byId(id)

  if (!playlist.value) {
    return triggerNotFound()
  }

  context.entity = playlist.value

  // Make sure these aren't shared among different playlists.
  selectedPlayables.value = []
  allPlayables.value = []

  currentState = getState(id)

  // (re)apply the filter based on the current state's keywords
  filterKeywords.value = currentState.filterKeywords

  await fetchDetails()

  // Another playlist opened meanwhile: its own run takes over.
  if (playlistId.value !== id) {
    return
  }

  fetchMirror()

  listConfig.reorderable = currentState.sortField === 'position' && playlist.value.permissions.edit
  listConfig.hasCustomOrderSort = !playlist.value.is_smart

  currentState.sortField ??= playlist.value?.is_smart ? 'title' : 'position'
  currentState.sortOrder ??= 'asc'

  sort(currentState.sortField, currentState.sortOrder)
})

onScreenActivated('Playlist', () => (playlistId.value = getRouteParam('id')!))

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'PLAYLIST'>(ContextMenu, event, {
    playlist: playlist.value!,
  })

const { toastSuccess } = useMessageToaster()

eventBus.on('WATCH_EXCLUSIONS_CHANGED', async () => {
  if (mirror.value) {
    await fetchDetails(true)
    fetchMirror()
  }
})
eventBus.on('PLAYLIST_UPDATED', async ({ id }) => id === playlistId.value && (await fetchDetails()))
eventBus.on('PLAYLIST_CONTENT_REMOVED', async ({ playlist: { id }, playables: removed }) => {
  if (id === playlistId.value) {
    allPlayables.value = differenceBy(allPlayables.value, removed, 'id')
  }
})
eventBus.on('PLAYLIST_DELETED', async ({ id }) => id === playlistId.value && go(url('home')))
</script>
