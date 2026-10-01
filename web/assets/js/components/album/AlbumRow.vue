<template>
  <article
    class="album-row list-table-row group h-(--m3-row-height) border-b border-(--schemes-outline-variant) hover:bg-(--schemes-surface-container-high) transition-colors"
    data-testid="album-row"
    :draggable="true"
    @contextmenu.prevent="onContextMenu"
    @dblclick.prevent.stop="goToAlbum"
    @dragstart="onDragStart"
  >
    <span class="name">
      <span class="size-(--m3-row-cover) flex-none">
        <AlbumOrArtistThumbnail :entity="album" size="sm" />
      </span>
      <a :href="url('albums.show', { id: album.id })" :title="album.name" class="truncate">{{ album.name }}</a>
    </span>
    <span v-if="shouldShowColumn('artist')" class="artist">
      <a
        v-if="artistStore.isStandard(album.artist_id)"
        :href="url('artists.show', { id: album.artist_id })"
        :title="album.artist_name"
        class="truncate"
      >
        {{ album.artist_name }}
      </a>
      <span v-else :title="album.artist_name" class="truncate">{{ album.artist_name }}</span>
    </span>
    <span v-if="shouldShowColumn('time')" class="time text-(--schemes-on-surface-variant) tabular-nums">
      {{ formatLength(album.length) }}
    </span>
    <span v-if="shouldShowColumn('year')" class="year text-(--schemes-on-surface-variant) tabular-nums">{{
      album.year ?? '—'
    }}</span>
    <span v-if="shouldShowColumn('rating')" class="rating">
      <StarRating :rateable="album" size="xs" />
    </span>
    <span v-if="shouldShowColumn('favorite')" class="favorite">
      <FavoriteButton :favorite="album.favorite" @toggle="emit('toggle-favorite', album)" />
    </span>
    <span class="extra">
      <M3IconButton icon="more_vert" label="More actions" @click="onContextMenu" />
    </span>
  </article>
</template>

<script lang="ts" setup>
import { useArtistStore } from '@/stores/artistStore'
import { useDraggable } from '@/composables/useDragAndDrop'
import { useRouter } from '@/composables/useRouter'
import { useContextMenu } from '@/composables/useContextMenu'
import { useTableColumnVisibility } from '@/composables/useTableColumnVisibility'
import { albumTableColumnConfig } from '@/config/tables'
import { secondsToHis } from '@/utils/formatters'
import { defineAsyncComponent } from '@/utils/helpers'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import StarRating from '@/components/ui/StarRating.vue'
import FavoriteButton from '@/components/ui/FavoriteButton.vue'
import AlbumOrArtistThumbnail from '@/components/ui/album-artist/AlbumOrArtistThumbnail.vue'

const artistStore = useArtistStore()

const props = defineProps<{ album: Album }>()

const emit = defineEmits<{
  (e: 'toggle-favorite', album: Album): void
}>()

const AlbumContextMenu = defineAsyncComponent(() => import('@/components/album/AlbumContextMenu.vue'))

const { go, url } = useRouter()
const { openContextMenu } = useContextMenu()
const { startDragging } = useDraggable('album')
const { shouldShowColumn } = useTableColumnVisibility(albumTableColumnConfig)

const formatLength = (seconds: number) => (seconds > 0 ? secondsToHis(seconds) : '—')

const onContextMenu = (event: MouseEvent) => openContextMenu<'ALBUM'>(AlbumContextMenu, event, { album: props.album })

const goToAlbum = () => go(url('albums.show', { id: props.album.id }))

const onDragStart = (event: DragEvent) => startDragging(event, props.album)
</script>
