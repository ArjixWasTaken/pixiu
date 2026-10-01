<template>
  <ScreenBase :tint-from="album?.cover">
    <template #header>
      <ScreenHeaderSkeleton v-if="loading && !album" role="status" aria-busy="true" aria-label="Loading" />

      <ScreenHeader v-if="album" :disabled="loading" :layout="songs.length ? headerLayout : 'collapsed'">
        {{ album.name }}

        <template #thumbnail>
          <AlbumThumbnail :entity="album" />
        </template>

        <template #meta>
          <a v-if="isStandardArtist" :href="url('artists.show', { id: album.artist_id })" class="artist">
            {{ album.artist_name }}
          </a>
          <span v-else class="text-(--schemes-on-surface)">{{ album.artist_name }}</span>
          <span v-if="album.year">{{ album.year }}</span>
          <span>{{ pluralize(songs, 'song') }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <SongListControls v-if="songs.length" :config @play-all="playAll" @play-selected="playSelected">
            <FavoriteButton :favorite="album.favorite" @toggle="toggleFavorite" />

            <M3IconButton icon="more_vert" label="More actions" @click="requestContextMenu" />
          </SongListControls>
        </template>
      </ScreenHeader>
    </template>

    <ScreenTabs v-if="album" class="screen-bleed" :class="loading && 'pointer-events-none'">
      <template #header>
        <nav>
          <ul>
            <li :class="activeTab === 'songs' && 'active'">
              <a href="#songs" @click.prevent="activeTab = 'songs'">Songs</a>
            </li>
            <li :class="activeTab === 'other-albums' && 'active'">
              <a href="#other-albums" @click.prevent="activeTab = 'other-albums'">Other albums</a>
            </li>
            <li v-if="useEncyclopedia" :class="activeTab === 'information' && 'active'">
              <a href="#information" @click.prevent="activeTab = 'information'">Information</a>
            </li>
          </ul>
        </nav>
      </template>

      <div v-show="activeTab === 'songs'" class="songs-pane">
        <SongListSkeleton v-if="loading" role="status" aria-busy="true" aria-label="Loading" />
        <SongList v-if="!loading && album" ref="songList" @sort="onSort" @press:enter="onPressEnter" @swipe="onSwipe" />
      </div>

      <div v-show="activeTab === 'other-albums'" class="albums-pane" data-testid="albums-pane">
        <template v-if="otherAlbums">
          <GridListView v-if="otherAlbums.length" class="scroll-mask-y">
            <AlbumCard v-for="otherAlbum in otherAlbums" :key="otherAlbum.id" :album="otherAlbum" />
          </GridListView>
          <p v-else class="p-6 text-(--schemes-on-surface-variant)">
            No other albums by {{ album.artist_name }} found in the library.
          </p>
        </template>
        <GridListView v-else>
          <AlbumCardSkeleton v-for="i in 6" :key="i" />
        </GridListView>
      </div>

      <div v-if="useEncyclopedia && album" v-show="activeTab === 'information'" class="info-pane">
        <AlbumInfo :album mode="full" />
        <AlbumMusicBrainz :album class="mt-10" />
      </div>
    </ScreenTabs>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { pluralize } from '@/utils/formatters'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { useRouter } from '@/composables/useRouter'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'
import { useContextMenu } from '@/composables/useContextMenu'
import { moveTabToHash, useHashTab } from '@/composables/useHash'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import AlbumThumbnail from '@/components/ui/album-artist/AlbumOrArtistThumbnail.vue'
import ScreenHeaderSkeleton from '@/components/ui/ScreenHeaderSkeleton.vue'
import SongListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenTabs from '@/components/ui/ArtistAlbumScreenTabs.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import GridListView from '@/components/ui/GridListView.vue'

const validTabs = ['songs', 'other-albums', 'information'] as const
type Tab = (typeof validTabs)[number]

const AlbumInfo = defineAsyncComponent(() => import('@/components/album/AlbumInfo.vue'))
const AlbumMusicBrainz = defineAsyncComponent(() => import('@/components/album/AlbumMusicBrainz.vue'))
const AlbumCard = defineAsyncComponent(() => import('@/components/album/AlbumCard.vue'))
const ContextMenu = defineAsyncComponent(() => import('@/components/album/AlbumContextMenu.vue'))
const AlbumCardSkeleton = defineAsyncComponent(() => import('@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'))
const FavoriteButton = defineAsyncComponent(() => import('@/components/ui/FavoriteButton.vue'))

const { getRouteParam, go, onScreenActivated, onRouteChanged, url, triggerNotFound } = useRouter()
const { PlayableListControls: SongListControls, config } = usePlayableListControls('Album')
const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('album-sort-field', 'track')
const sortOrder = useUserStorage<SortOrder>('album-sort-order', 'asc')
const { useMusicBrainz } = useThirdPartyServices()
const { openContextMenu } = useContextMenu()

const activeTab = useHashTab(validTabs, 'songs')

const album = ref<Album | undefined>()
const songs = ref<Song[]>([])
const loading = ref(false)
const otherAlbums = ref<Album[] | undefined>()
const info = ref<ArtistInfo | undefined>()

const {
  PlayableList: SongList,
  headerLayout,
  playableList: songList,
  duration,
  context,
  sort,
  onPressEnter,
  playAll,
  playSelected,
  onSwipe,
} = usePlayableList(songs, { type: 'Album' })

const useEncyclopedia = useMusicBrainz

const isStandardArtist = computed(() => {
  if (!album.value) {
    return true
  }

  return !artistStore.isVarious(album.value.artist_name) && !artistStore.isUnknown(album.value.artist_name)
})

const toggleFavorite = () => albumStore.toggleFavorite(album.value!)

const fetchScreenData = async () => {
  if (loading.value) {
    return
  }

  const id = getRouteParam('id')

  // Links from before the tab lived in the hash: `/albums/al-1/other-albums`.
  const legacyTab = getRouteParam<Tab>('tab')

  if (legacyTab && validTabs.includes(legacyTab)) {
    moveTabToHash(legacyTab)
    activeTab.value = legacyTab
  }

  album.value = undefined
  info.value = undefined
  otherAlbums.value = undefined

  loading.value = true

  try {
    ;[album.value, songs.value] = await Promise.all([albumStore.resolve(id), playableStore.fetchSongsForAlbum(id)])

    if (!album.value) {
      // If the album does not exist, redirect to the album list.
      triggerNotFound()
      return
    }

    context.entity = album.value

    sort(sortField.value, sortOrder.value)
  } catch (error: unknown) {
    if ((error as any)?.status === 404) {
      triggerNotFound()
      return
    }

    useErrorHandler('dialog').handleHttpError(error)
  } finally {
    loading.value = false
  }
}

/** The artist's other albums, the first time their tab shows. */
const fetchOtherAlbums = async () => {
  const shown = album.value

  if (!shown || otherAlbums.value) {
    return
  }

  try {
    const albums = await albumStore.fetchForArtist(shown.artist_id)

    // Another album may have opened meanwhile.
    if (album.value === shown) {
      otherAlbums.value = albums.filter(({ id }) => id !== shown.id)
    }
  } catch (error: unknown) {
    useErrorHandler('dialog').handleHttpError(error)
  }
}

watch([activeTab, album], ([tab]) => tab === 'other-albums' && fetchOtherAlbums())

const onSort = (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  sortField.value = field
  sortOrder.value = order
}

onScreenActivated('Album', () => fetchScreenData())
onRouteChanged(route => route.name === 'albums.show' && fetchScreenData())

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'ALBUM'>(ContextMenu, event, {
    album: album.value!,
  })

eventBus.on('SONGS_UPDATED', result => {
  // After songs are updated, check if the current album still exists.
  // If it doesn't, redirect to the album list.
  if (result.removed.album_ids.includes(album.value!.id)) {
    go(url('albums.index'))
  }
})
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.screen-header :deep(.play-icon) {
  @apply scale-[2];
}
</style>
