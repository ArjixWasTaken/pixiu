<template>
  <BaseCard
    v-if="showing"
    :entity="album"
    :href="url('albums.show', { id: album.id })"
    :title="`${album.name} by ${album.artist_name}`"
    class="group"
    @contextmenu="requestContextMenu"
    @dblclick="shuffle"
    @dragstart="onDragStart"
  >
    <template #thumbnail>
      <CardThumbnail :entity="album" @toggle-favorite="toggleFavorite" @context-menu="requestContextMenu" />
    </template>

    <template #name>
      <a :href="url('albums.show', { id: album.id })" class="m3-title-medium title" data-testid="name">
        {{ album.name }}
      </a>
      <p class="m3-body-medium subtitle">{{ subtitle }}</p>
    </template>
  </BaseCard>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { useAlbumStore } from '@/stores/albumStore'
import { usePlayableStore } from '@/stores/playableStore'
import { useViewport } from '@/composables/useViewport'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { playback } from '@/services/playbackManager'
import { useContextMenu } from '@/composables/useContextMenu'
import { defineAsyncComponent } from '@/utils/helpers'

import BaseCard from '@/components/ui/album-artist/AlbumOrArtistCard.vue'
import CardThumbnail from '@/components/ui/album-artist/AlbumOrArtistCardThumbnail.vue'

const albumStore = useAlbumStore()
const playableStore = usePlayableStore()

const props = withDefaults(
  defineProps<{
    album: Album
    showReleaseYear?: boolean
  }>(),
  {
    showReleaseYear: false,
  },
)

const AlbumContextMenu = defineAsyncComponent(() => import('@/components/album/AlbumContextMenu.vue'))

const { go, url } = useRouter()
const { startDragging } = useDraggable('album')
const { openContextMenu } = useContextMenu()

const { album } = toRefs(props)

const { isMobile } = useViewport()

/** "Artist · year"; phones show only the artist. */
const subtitle = computed(() =>
  isMobile.value || !album.value.year ? album.value.artist_name : `${album.value.artist_name} · ${album.value.year}`,
)
const showing = computed(() => !albumStore.isUnknown(album.value))

const shuffle = async () => {
  go(url('queue'))
  await playback().queueAndPlay(await playableStore.fetchSongsForAlbum(album.value), true /* shuffled */)
}

const toggleFavorite = () => albumStore.toggleFavorite(album.value)

const onDragStart = (event: DragEvent) => startDragging(event, album.value)

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'ALBUM'>(AlbumContextMenu, event, {
    album: album.value,
  })
</script>
