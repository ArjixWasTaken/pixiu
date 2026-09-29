<template>
  <BaseCard
    v-if="showing"
    :entity="artist"
    :href="url('artists.show', { id: artist.id })"
    :title="artist.name"
    @contextmenu="requestContextMenu"
    @dblclick="shuffle"
    @dragstart="onDragStart"
  >
    <template #thumbnail>
      <CardThumbnail :entity="artist" @toggle-favorite="toggleFavorite" @context-menu="requestContextMenu" />
    </template>

    <template #name>
      <a :href="url('artists.show', { id: artist.id })" class="m3-title-medium title" data-testid="name">
        {{ artist.name }}
      </a>
      <p class="m3-body-medium subtitle">Artist</p>
    </template>
  </BaseCard>
</template>

<script lang="ts" setup>
import { computed, toRefs } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { playback } from '@/services/playbackManager'
import { useContextMenu } from '@/composables/useContextMenu'

import BaseCard from '@/components/ui/album-artist/AlbumOrArtistCard.vue'
import CardThumbnail from '@/components/ui/album-artist/AlbumOrArtistCardThumbnail.vue'

const props = defineProps<{ artist: Artist }>()

const ContextMenu = defineAsyncComponent(() => import('@/components/artist/ArtistContextMenu.vue'))

const { go, url } = useRouter()
const { startDragging } = useDraggable('artist')
const { openContextMenu } = useContextMenu()

const { artist } = toRefs(props)

const showing = computed(() => artistStore.isStandard(artist.value))

const shuffle = async () => {
  playback().queueAndPlay(await playableStore.fetchSongsForArtist(artist.value), true /* shuffled */)
  go(url('queue'))
}

const toggleFavorite = () => artistStore.toggleFavorite(artist.value)

const onDragStart = (event: DragEvent) => startDragging(event, artist.value)

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'ARTIST'>(ContextMenu, event, {
    artist: artist.value,
  })
</script>
