<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed" :disabled="loading">
        Artists
        <template #controls>
          <div class="flex gap-2 items-center">
            <M3Chip
              :selected="preferences.artists_favorites_only"
              class="shrink-0"
              variant="filter"
              @click.prevent="toggleFavoritesOnly"
            >
              Favorites only
            </M3Chip>

            <ArtistListSorter
              v-if="preferences.artists_view_mode !== 'table'"
              :field="preferences.artists_sort_field"
              :order="preferences.artists_sort_order"
              @sort="sort"
            />

            <ViewModeSwitch v-if="!isMobile" v-model="preferences.artists_view_mode" secondary="table" />
          </div>
        </template>
      </ScreenHeader>
    </template>

    <ScreenEmptyState v-if="libraryEmpty">
      <template #icon>
        <M3Icon name="mic_off" />
      </template>
      No artists found.
      <EmptyLibraryHint />
    </ScreenEmptyState>

    <LoadFailedState v-else-if="loadFailed" what="artists" @retry="refetch" />

    <ScreenEmptyState v-else-if="noFavoriteArtists">
      <template #icon>
        <M3Icon name="mic_off" />
      </template>
      No favorite artists.
    </ScreenEmptyState>

    <template v-else>
      <div
        v-if="showSkeletons && preferences.artists_view_mode === 'table'"
        class="screen-bleed flex flex-col"
        role="status"
        aria-busy="true"
        aria-label="Loading"
      >
        <ArtistTableRowSkeleton v-for="i in 12" :key="i" />
      </div>
      <div
        v-else-if="showSkeletons"
        :style="{ gridTemplateColumns: `repeat(auto-fill, minmax(${isMobile ? 140 : 180}px, 1fr))` }"
        class="screen-bleed virtual-card-grid grid"
        role="status"
        aria-busy="true"
        aria-label="Loading"
      >
        <ArtistCardSkeleton v-for="i in 10" :key="i" round />
      </div>
      <div class="screen-bleed flex-1 flex flex-col min-h-0" v-else>
        <ArtistTable
          v-if="preferences.artists_view_mode === 'table'"
          :artists="displayedArtists"
          :field="preferences.artists_sort_field"
          :order="preferences.artists_sort_order"
          @sort="sort"
          @toggle-favorite="toggleFavorite"
          @scrolled-to-end="fetchArtists"
        />
        <ArtistGrid v-else ref="grid" :artists="gridArtists" @scrolled-to-end="fetchArtists" />
      </div>
    </template>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue'
import { useArtistStore } from '@/stores/artistStore'
import { useCommonStore } from '@/stores/commonStore'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useListPages } from '@/composables/useListPages'
import { useViewport } from '@/composables/useViewport'

import ArtistCardSkeleton from '@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'
import ArtistGrid from '@/components/artist/ArtistGrid.vue'
import ArtistTable from '@/components/artist/ArtistTable.vue'
import ArtistTableRowSkeleton from '@/components/artist/ArtistTableRowSkeleton.vue'
import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ViewModeSwitch from '@/components/ui/ViewModeSwitch.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import LoadFailedState from '@/components/ui/LoadFailedState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import ArtistListSorter from '@/components/artist/ArtistListSorter.vue'
import M3Chip from '@/components/m3/M3Chip.vue'
import EmptyLibraryHint from '@/components/ui/EmptyLibraryHint.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const artistStore = useArtistStore()
const commonStore = useCommonStore()
const preferences = usePreferenceStore()

const { isMobile } = useViewport()
const grid = ref<InstanceType<typeof ArtistGrid>>()
const libraryEmpty = computed(() => commonStore.state.song_length === 0)

// Each sort and filter is a list of its own; changing one starts it from its first page.
const {
  items: artists,
  isFetching: loading,
  hasNextPage: moreArtistsAvailable,
  fetchMore: fetchArtists,
  loadFailed,
  refetch,
} = useListPages(
  () => [
    'artists',
    {
      sort: preferences.artists_sort_field,
      order: preferences.artists_sort_order,
      favoritesOnly: preferences.artists_favorites_only,
    },
  ],
  cursor =>
    artistStore.paginate({
      favorites_only: preferences.artists_favorites_only,
      cursor,
      sort: preferences.artists_sort_field,
      order: preferences.artists_sort_order,
    }),
  // Re-sorting keeps the grid (and the focus on the sort button) until the new order comes.
  { enabled: () => !libraryEmpty.value, keepPrevious: true },
)

const displayedArtists = computed(() =>
  preferences.artists_favorites_only ? artists.value.filter((a: Artist) => a.favorite) : artists.value,
)

// Unknown and Various Artists have no card (ArtistCard shows none), so they take no place in the grid; the table lists them.
const gridArtists = computed(() => displayedArtists.value.filter(artist => artistStore.isStandard(artist)))

const noFavoriteArtists = computed(
  () =>
    !loading.value &&
    preferences.artists_favorites_only &&
    displayedArtists.value.length === 0 &&
    !moreArtistsAvailable.value,
)
const showSkeletons = computed(() => loading.value && artists.value.length === 0)

const sort = (field: ArtistListSortField, order: SortOrder) => {
  preferences.artists_sort_field = field
  preferences.artists_sort_order = order
  grid.value?.scrollToTop()
}

const toggleFavorite = (artist: Artist) => artistStore.toggleFavorite(artist)

const toggleFavoritesOnly = () => {
  preferences.artists_favorites_only = !preferences.artists_favorites_only
  grid.value?.scrollToTop()
}
</script>
