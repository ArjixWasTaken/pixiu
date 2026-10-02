<template>
  <article
    class="artist-row list-table-row group h-(--m3-row-height) border-b border-(--schemes-outline-variant) hover:bg-(--schemes-surface-container-high) transition-colors"
    data-testid="artist-row"
    :draggable="true"
    @contextmenu.prevent="onContextMenu"
    @dblclick.prevent.stop="goToArtist"
    @dragstart="onDragStart"
  >
    <span class="name">
      <span class="size-(--m3-row-cover) flex-none">
        <AlbumOrArtistThumbnail :entity="artist" size="sm" />
      </span>
      <a :href="url('artists.show', { id: artist.id })" class="truncate">{{ artist.name }}</a>
    </span>
    <span v-if="shouldShowColumn('rating')" class="rating">
      <StarRating :rateable="artist" size="xs" />
    </span>
    <span v-if="shouldShowColumn('favorite')" class="favorite">
      <FavoriteButton :favorite="artist.favorite" @toggle="emit('toggle-favorite', artist)" />
    </span>
    <span class="extra">
      <M3IconButton icon="more_vert" label="More actions" @click="onContextMenu" />
    </span>
  </article>
</template>

<script lang="ts" setup>
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { useContextMenu } from '@/composables/useContextMenu'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { artistTableColumnConfig } from '@/config/tables'
import { defineAsyncComponent } from '@/utils/helpers'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import StarRating from '@/components/ui/StarRating.vue'
import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import AlbumOrArtistThumbnail from '@/components/ui/album-artist/AlbumOrArtistThumbnail.vue'

const props = defineProps<{ artist: Artist }>()

const emit = defineEmits<{
  (e: 'toggle-favorite', artist: Artist): void
}>()

const ArtistContextMenu = defineAsyncComponent(() => import('@/components/artist/ArtistContextMenu.vue'))

const { go, url } = useRouter()
const { openContextMenu } = useContextMenu()
const { startDragging } = useDraggable('artist')
const { shouldShowColumn } = useTableColumnVisibility(artistTableColumnConfig)

const onContextMenu = (event: MouseEvent) =>
  openContextMenu<'ARTIST'>(ArtistContextMenu, event, { artist: props.artist })

const goToArtist = () => go(url('artists.show', { id: props.artist.id }))

const onDragStart = (event: DragEvent) => startDragging(event, props.artist)
</script>
