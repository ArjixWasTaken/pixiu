<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed" :disabled="loading">
        Genres

        <template #controls>
          <div class="flex gap-2 items-center">
            <ListFilter />

            <GenreListSorter
              :field="preferences.genres_sort_field"
              :order="preferences.genres_sort_order"
              @sort="sort"
            />
          </div>
        </template>
      </ScreenHeader>
    </template>

    <ScreenEmptyState v-if="libraryEmpty">
      <template #icon>
        <M3Icon name="category" :size="96" />
      </template>
      No genres found.
      <EmptyLibraryHint />
    </ScreenEmptyState>

    <ScreenEmptyState v-else-if="!loading && !genres.length" data-testid="no-genres">
      <template #icon>
        <M3Icon name="category" :size="96" />
      </template>
      No genres yet.
      <span class="block secondary"> Genres come from the songs' tags and MusicBrainz, as albums are looked up. </span>
    </ScreenEmptyState>

    <template v-else>
      <ul v-if="!loading" class="genre-list grid gap-3 pt-2">
        <GenreCard v-for="(genre, index) in displayedGenres" :key="genre.id" :genre :tone="index" />
      </ul>

      <ul v-else class="genre-list grid gap-3 pt-2" role="status" aria-busy="true" aria-label="Loading">
        <GenreCardSkeleton v-for="key in 11" :key />
      </ul>
    </template>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { computed, onMounted, provide, ref } from 'vue'
import { useCommonStore } from '@/stores/commonStore'
import { useGenreStore } from '@/stores/genreStore'
import { useErrorHandler } from '@/composables/useErrorHandler'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useFuzzySearch } from '@/composables/useFuzzySearch'
import { FilterKeywordsKey } from '@/config/symbols'
import { orderBy } from 'lodash-es'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import GenreCardSkeleton from '@/components/genre/GenreCardSkeleton.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import GenreCard from '@/components/genre/GenreCard.vue'
import ListFilter from '@/components/ui/ListFilter.vue'
import GenreListSorter from '@/components/genre/GenreListSorter.vue'
import EmptyLibraryHint from '@/components/ui/EmptyLibraryHint.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const commonStore = useCommonStore()
const genreStore = useGenreStore()
const preferences = usePreferenceStore()

const { handleHttpError } = useErrorHandler()

const genres = ref<Genre[]>([])
const keywords = ref('')
const loading = ref(false)

const fuzzy = useFuzzySearch<Genre>(genres, ['name'])

provide(FilterKeywordsKey, keywords)

const displayedGenres = computed(() => {
  const all = keywords.value ? fuzzy.search(keywords.value) : genres.value

  if (preferences.genres_sort_field === 'name') {
    // if sorted by name, ensure 'No genre' is always on top
    return orderBy(all, [genre => (genre.name ? 1 : 0), 'name'], ['asc', preferences.genres_sort_order])
  }

  return orderBy(all, preferences.genres_sort_field, preferences.genres_sort_order)
})

const libraryEmpty = computed(() => commonStore.state.song_length === 0)

const fetchGenres = async () => {
  if (loading.value) {
    return
  }

  try {
    loading.value = true
    genres.value = await genreStore.fetchAll()
  } catch (error: unknown) {
    handleHttpError(error)
  } finally {
    loading.value = false
  }
}

const sort = (field: GenreListSortField, order: SortOrder) => {
  preferences.genres_sort_field = field
  preferences.genres_sort_order = order
}

onMounted(async () => {
  if (libraryEmpty.value) {
    return
  }

  await fetchGenres()
})
</script>

<style lang="postcss">
.genre-list {
  content-visibility: auto;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));

  /* Two to a row on a phone, not one long column. */
  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  }
}
</style>
