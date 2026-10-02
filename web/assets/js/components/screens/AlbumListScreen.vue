<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed" :disabled="loading">
        Albums
        <template #controls>
          <div class="flex gap-2 items-center">
            <M3Chip
              :selected="preferences.albums_favorites_only"
              class="shrink-0"
              variant="filter"
              @click.prevent="toggleFavoritesOnly"
            >
              Favorites only
            </M3Chip>

            <AlbumListSorter
              v-if="preferences.albums_view_mode !== 'table'"
              :field="preferences.albums_sort_field"
              :order="preferences.albums_sort_order"
              @sort="sort"
            />

            <ViewModeSwitch v-if="!isMobile" v-model="preferences.albums_view_mode" secondary="table" />
          </div>
        </template>
      </ScreenHeader>
    </template>

    <ScreenEmptyState v-if="libraryEmpty">
      <template #icon>
        <M3Icon name="album" />
      </template>
      No albums found.
      <EmptyLibraryHint />
    </ScreenEmptyState>

    <LoadFailedState v-else-if="loadFailed" what="albums" @retry="refetch" />

    <ScreenEmptyState v-else-if="noFavoriteAlbums">
      <template #icon>
        <M3Icon name="album" />
      </template>
      No favorite albums.
    </ScreenEmptyState>

    <template v-else>
      <div
        v-if="showSkeletons && preferences.albums_view_mode === 'table'"
        class="screen-bleed flex flex-col"
        role="status"
        aria-busy="true"
        aria-label="Loading"
      >
        <AlbumTableRowSkeleton v-for="i in 12" :key="i" />
      </div>
      <div
        v-else-if="showSkeletons"
        :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${isMobile ? 140 : 180}px, 1fr))` }"
        class="screen-bleed virtual-card-grid grid"
        role="status"
        aria-busy="true"
        aria-label="Loading"
      >
        <AlbumCardSkeleton v-for="i in 10" :key="i" />
      </div>
      <div class="screen-bleed flex-1 flex flex-col min-h-0" v-else>
        <AlbumTable
          v-if="preferences.albums_view_mode === 'table'"
          :albums="displayedAlbums"
          :field="preferences.albums_sort_field"
          :order="preferences.albums_sort_order"
          @sort="sort"
          @toggle-favorite="toggleFavorite"
          @scrolled-to-end="fetchAlbums"
        />
        <AlbumGrid
          v-else
          ref="grid"
          :albums="gridAlbums"
          :show-release-year="preferences.albums_sort_field === 'year'"
          @scrolled-to-end="fetchAlbums"
        />
      </div>
    </template>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useAlbumStore } from '@/stores/albumStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useListPages } from '@/composables/useListPages'
import { useViewport } from '@/composables/useViewport'

import AlbumCardSkeleton from '@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'
import AlbumGrid from '@/components/album/AlbumGrid.vue'
import AlbumTable from '@/components/album/AlbumTable.vue'
import AlbumTableRowSkeleton from '@/components/album/AlbumTableRowSkeleton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ViewModeSwitch from '@/components/ui/ViewModeSwitch.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import LoadFailedState from '@/components/ui/LoadFailedState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import AlbumListSorter from '@/components/album/AlbumListSorter.vue'
import M3Chip from '@/components/m3/M3Chip.vue'
import EmptyLibraryHint from '@/components/ui/EmptyLibraryHint.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const albumStore = useAlbumStore()
const commonStore = useCommonStore()
const preferences = usePreferenceStore()

const { isMobile } = useViewport()
const grid = ref<InstanceType<typeof AlbumGrid>>()
const libraryEmpty = computed(() => commonStore.state.song_length === 0)

// Each sort and filter is a list of its own; changing one starts it from its first page.
const {
  items: albums,
  isFetching: loading,
  hasNextPage: moreAlbumsAvailable,
  fetchMore: fetchAlbums,
  loadFailed,
  refetch,
} = useListPages(
  () => [
    'albums',
    {
      sort: preferences.albums_sort_field,
      order: preferences.albums_sort_order,
      favoritesOnly: preferences.albums_favorites_only,
    },
  ],
  cursor =>
    albumStore.paginate({
      favorites_only: preferences.albums_favorites_only,
      cursor,
      sort: preferences.albums_sort_field,
      order: preferences.albums_sort_order,
    }),
  // Re-sorting keeps the grid (and the focus on the sort button) until the new order comes.
  { enabled: () => !libraryEmpty.value, keepPrevious: true },
)

const displayedAlbums = computed(() =>
  preferences.albums_favorites_only ? albums.value.filter((a: Album) => a.favorite) : albums.value,
)

// Unknown Album has no card (AlbumCard shows none), so it takes no place in the grid; the table lists it.
const gridAlbums = computed(() => displayedAlbums.value.filter(album => !albumStore.isUnknown(album)))

const noFavoriteAlbums = computed(
  () =>
    !loading.value &&
    preferences.albums_favorites_only &&
    displayedAlbums.value.length === 0 &&
    !moreAlbumsAvailable.value,
)
const showSkeletons = computed(() => loading.value && albums.value.length === 0)

const sort = (field: AlbumListSortField, order: SortOrder) => {
  preferences.albums_sort_field = field
  preferences.albums_sort_order = order
  grid.value?.scrollToTop()
}

const toggleFavorite = (album: Album) => albumStore.toggleFavorite(album)

const toggleFavoritesOnly = () => {
  preferences.albums_favorites_only = !preferences.albums_favorites_only
  grid.value?.scrollToTop()
}
</script>
