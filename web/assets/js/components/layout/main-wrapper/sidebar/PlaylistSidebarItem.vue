<template>
  <SidebarItem
    :class="{ droppable }"
    :href="href"
    class="playlist select-none"
    :draggable="!isTouch"
    :active
    :icon
    @dblclick="onDblClick"
    @contextmenu="onContextMenu"
    @dragleave="onDragLeave"
    @dragover="onDragOver"
    @dragstart.stop="onDragStart"
    @drop="onDrop"
  >
    {{ list.name }}
  </SidebarItem>
</template>

<script lang="ts" setup>
import { useViewport } from '@/composables/useViewport'
import { computed, inject, ref, toRefs } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { usePlayableStore } from '@/stores/playableStore'
import { useRecentlyPlayedStore } from '@/stores/recentlyPlayedStore'
import { useRouter } from '@/composables/useRouter'
import { useDraggable, useDroppable } from '@/composables/useDragAndDrop'
import { usePlaylistContentManagement } from '@/composables/usePlaylistContentManagement'
import { useContextMenu } from '@/composables/useContextMenu'
import { playback } from '@/services/playbackManager'
import { DraggedPlaylistKey } from '@/config/symbols'

import SidebarItem from '@/components/layout/main-wrapper/sidebar/SidebarItem.vue'

const playableStore = usePlayableStore()
const recentlyPlayedStore = useRecentlyPlayedStore()

const { isTouch } = useViewport()

const props = defineProps<{ list: PlaylistLike }>()

const PlaylistContextMenu = defineAsyncComponent(() => import('@/components/playlist/PlaylistContextMenu.vue'))

const { url, isCurrentScreen, getRouteParam } = useRouter()
const { startDragging } = useDraggable('playlist')
const { acceptsDrop, resolveDroppedItems } = useDroppable(['playables', 'album', 'artist'])
const { openContextMenu } = useContextMenu()

const draggedPlaylist = inject(DraggedPlaylistKey, ref<Playlist | null>(null))

const droppable = ref(false)

const { addToPlaylist } = usePlaylistContentManagement()

const { list } = toRefs(props)

const isPlaylist = (list: PlaylistLike): list is Playlist => 'id' in list
const isFavoriteList = (list: PlaylistLike): list is FavoriteList => list.name === 'Favorites'
const isRecentlyPlayedList = (list: PlaylistLike): list is RecentlyPlayedList => list.name === 'Recently played'

const active = computed(() => {
  return (
    (isCurrentScreen('Favorites') && isFavoriteList(list.value)) ||
    (isCurrentScreen('RecentlyPlayed') && isRecentlyPlayedList(list.value)) ||
    (isCurrentScreen('Playlist') && (list.value as Playlist).id === getRouteParam('id'))
  )
})

/** Smart playlists, mirrors of watched playlists, and plain ones. */
const icon = computed(() => {
  if (isRecentlyPlayedList(list.value)) {
    return 'history'
  }

  if (isFavoriteList(list.value)) {
    return 'favorite'
  }

  if (list.value.is_smart) {
    return 'auto_awesome'
  }

  return list.value.permissions.edit ? 'queue_music' : 'sync'
})

const href = computed(() => {
  if (isPlaylist(list.value)) {
    return url('playlists.show', { id: list.value.id })
  }

  if (isFavoriteList(list.value)) {
    return url('favorites')
  }

  if (isRecentlyPlayedList(list.value)) {
    return url('recently-played')
  }

  throw new Error('Invalid playlist-like type.')
})

const contentEditable = computed(() => {
  if (isRecentlyPlayedList(list.value)) {
    return false
  }
  if (isFavoriteList(list.value)) {
    return true
  }

  return !list.value.is_smart
})

const onContextMenu = (event: MouseEvent) => {
  if (isPlaylist(list.value)) {
    event.preventDefault()
    openContextMenu<'PLAYLIST'>(PlaylistContextMenu, event, {
      playlist: list.value,
    })
  }
}

const onDblClick = async () => {
  let playables: Playable[]

  if (isFavoriteList(list.value)) {
    playables = await playableStore.fetchFavorites()
  } else if (isRecentlyPlayedList(list.value)) {
    playables = await recentlyPlayedStore.fetch()
  } else {
    playables = await playableStore.fetchForPlaylist(list.value as Playlist)
  }

  if (playables.length) {
    playback().queueAndPlay(playables)
  }
}

const onDragStart = (event: DragEvent) => {
  if (!isPlaylist(list.value)) {
    return
  }

  startDragging(event, list.value)
  draggedPlaylist.value = list.value
}

const onDragOver = (event: DragEvent) => {
  if (!contentEditable.value || !acceptsDrop(event)) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
  droppable.value = true
}

const onDragLeave = (event: DragEvent) => {
  if (!droppable.value) {
    return
  }

  event.stopPropagation()
  droppable.value = false
}

const onDrop = async (event: DragEvent) => {
  if (!contentEditable.value || !acceptsDrop(event)) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
  droppable.value = false

  const playables = await resolveDroppedItems(event)

  if (!playables?.length) {
    return
  }

  if (isFavoriteList(list.value)) {
    await playableStore.favorite(playables)
  } else if (isPlaylist(list.value)) {
    await addToPlaylist(list.value, playables)
  }
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.droppable :deep(a) {
  @apply ring-2 ring-offset-0 ring-(--schemes-primary) cursor-copy;
}
</style>
