<template>
  <ScreenBase :tint-from="artist?.image">
    <template #header>
      <ScreenHeaderSkeleton v-if="loading && !artist" role="status" aria-busy="true" aria-label="Loading" />

      <ScreenHeader v-if="artist" :disabled="loading" :layout="songs.length ? headerLayout : 'collapsed'">
        {{ artist.name }}

        <template #thumbnail>
          <ArtistThumbnail :entity="artist" />
        </template>

        <template #meta>
          <span>{{ pluralize(albumCount, 'album') }}</span>
          <span>{{ pluralize(songs, 'song') }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <SongListControls v-if="songs.length" :config @play-all="playAll" @play-selected="playSelected">
            <FavoriteButton :favorite="artist.favorite" @toggle="toggleFavorite" />
            <M3IconButton icon="more_vert" label="More actions" @click="requestContextMenu" />
          </SongListControls>
        </template>
      </ScreenHeader>
    </template>

    <ScreenTabs v-if="artist" class="screen-bleed" :class="loading && 'pointer-events-none'">
      <template #header>
        <nav>
          <ul>
            <li :class="activeTab === 'songs' && 'active'">
              <a href="#songs" @click.prevent="activeTab = 'songs'">Songs</a>
            </li>
            <li :class="activeTab === 'albums' && 'active'">
              <a href="#albums" @click.prevent="activeTab = 'albums'">Albums</a>
            </li>
            <li v-if="useEncyclopedia" :class="activeTab === 'information' && 'active'">
              <a href="#information" @click.prevent="activeTab = 'information'">Information</a>
            </li>
          </ul>
        </nav>
      </template>

      <div v-show="activeTab === 'songs'" class="songs-pane">
        <SongListSkeleton v-if="loading" role="status" aria-busy="true" aria-label="Loading" />
        <SongList
          v-if="!loading && artist"
          ref="songList"
          @sort="onSort"
          @press:enter="onPressEnter"
          @swipe="onSwipe"
        />
      </div>

      <div v-show="activeTab === 'albums'" class="albums-pane">
        <GridListView class="scroll-mask-y">
          <template v-if="albums">
            <AlbumCard v-for="album in albums" :key="album.id" :album :show-release-year="true" />
          </template>
          <template v-else>
            <AlbumCardSkeleton v-for="i in 6" :key="i" />
          </template>
        </GridListView>
      </div>

      <div v-if="useEncyclopedia && artist" v-show="activeTab === 'information'" class="info-pane">
        <ArtistInfo :artist mode="full" />
      </div>
    </ScreenTabs>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, ref, watch } from 'vue'
import { defineAsyncComponent } from '@/utils/helpers'
import { eventBus } from '@/utils/eventBus'
import { pluralize } from '@/utils/formatters'
import { albumStore } from '@/stores/albumStore'
import { artistStore } from '@/stores/artistStore'
import { playableStore } from '@/stores/playableStore'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useUserStorage } from '@/composables/useUserStorage'
import { useThirdPartyServices } from '@/composables/useThirdPartyServices'
import { useRouter } from '@/composables/useRouter'
import { useContextMenu } from '@/composables/useContextMenu'
import { useHashTab } from '@/composables/useHash'
import { isNotFound } from '@/services/subsonic'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ArtistThumbnail from '@/components/ui/album-artist/AlbumOrArtistThumbnail.vue'
import ScreenHeaderSkeleton from '@/components/ui/ScreenHeaderSkeleton.vue'
import SongListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenTabs from '@/components/ui/ArtistAlbumScreenTabs.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import GridListView from '@/components/ui/GridListView.vue'

const ArtistInfo = defineAsyncComponent(() => import('@/components/artist/ArtistInfo.vue'))
const AlbumCard = defineAsyncComponent(() => import('@/components/album/AlbumCard.vue'))
const AlbumCardSkeleton = defineAsyncComponent(() => import('@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'))
const FavoriteButton = defineAsyncComponent(() => import('@/components/ui/FavoriteButton.vue'))
const ArtistContextMenu = defineAsyncComponent(() => import('@/components/artist/ArtistContextMenu.vue'))

const validTabs = ['songs', 'albums', 'information'] as const
type Tab = (typeof validTabs)[number]

const { PlayableListControls: SongListControls, config } = usePlayableListControls('Artist')
const { useMusicBrainz } = useThirdPartyServices()
const { getRouteParam, go, onScreenActivated, onRouteChanged, url, triggerNotFound } = useRouter()
const { openContextMenu } = useContextMenu()
const sortField = useUserStorage<MaybeArray<PlayableListSortField>>('artist-sort-field', 'track')
const sortOrder = useUserStorage<SortOrder>('artist-sort-order', 'asc')

const activeTab = useHashTab(validTabs, 'songs')

const artist = ref<Artist>()
const songs = ref<Song[]>([])
const loading = ref(false)
const albums = ref<Album[] | undefined>()

const {
  PlayableList: SongList,
  headerLayout,
  playableList: songList,
  context,
  duration,
  sort,
  onPressEnter,
  playAll,
  playSelected,
  onSwipe,
} = usePlayableList(songs, { type: 'Artist' })

const useEncyclopedia = useMusicBrainz

const albumCount = computed(() => {
  const albums = new Set()
  songs.value.forEach(song => albums.add(song.album_id))
  return albums.size
})

const toggleFavorite = () => artistStore.toggleFavorite(artist.value!)

const fetchScreenData = async () => {
  if (loading.value) {
    return
  }

  const id = getRouteParam('id')

  albums.value = undefined
  loading.value = true

  try {
    ;[artist.value, songs.value] = await Promise.all([artistStore.resolve(id), playableStore.fetchSongsForArtist(id)])

    if (!artist.value) {
      triggerNotFound()
      return
    }

    context.entity = artist.value

    sort(sortField.value, sortOrder.value)
  } catch (error: unknown) {
    if (isNotFound(error)) {
      triggerNotFound()
      return
    }

    useErrorHandler('dialog').handleHttpError(error)
  } finally {
    loading.value = false
  }
}

/** The artist's albums, the first time their tab shows. */
const fetchAlbums = async () => {
  const shown = artist.value

  if (!shown || albums.value) {
    return
  }

  try {
    const found = await albumStore.fetchForArtist(shown)

    // Another artist may have opened meanwhile.
    if (artist.value === shown) {
      albums.value = found
    }
  } catch (error: unknown) {
    useErrorHandler('dialog').handleHttpError(error)
  }
}

watch([activeTab, artist], ([tab]) => tab === 'albums' && fetchAlbums())

const onSort = (field: MaybeArray<PlayableListSortField>, order: SortOrder) => {
  sortField.value = field
  sortOrder.value = order
}

onScreenActivated('Artist', () => fetchScreenData())
onRouteChanged(route => route.name === 'artists.show' && fetchScreenData())

const requestContextMenu = (event: MouseEvent) =>
  openContextMenu<'ARTIST'>(ArtistContextMenu, event, {
    artist: artist.value!,
  })

eventBus.on('SONGS_UPDATED', result => {
  // After songs are updated, check if the current artist still exists.
  // If not, redirect to the artist index screen.
  if (result.removed.artist_ids.includes(artist.value!.id)) {
    go(url('artists.index'))
  }
})
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.screen-header :deep(.play-icon) {
  @apply scale-[2];
}
</style>
