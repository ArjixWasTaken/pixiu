<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader layout="collapsed">
        <span v-if="q">Searching for {{ q }}</span>
        <span v-else>Search</span>
      </ScreenHeader>
    </template>

    <div v-if="q" class="space-y-8">
      <PlayableExcerptResultsBlock
        :playables="excerpt.playables"
        :query="q"
        :searching
        data-testid="playable-excerpts"
      />
      <ArtistResultsBlock :artists="excerpt.artists" :searching data-testid="artist-excerpts" />
      <AlbumResultsBlock :albums="excerpt.albums" :searching data-testid="album-excerpts" />

      <M3Button :href="discoverUrl" icon="travel_explore" variant="tonal">
        Search YouTube Music for “{{ q }}”
      </M3Button>
    </div>

    <ScreenEmptyState v-else>
      <template #icon>
        <M3Icon :size="64" name="search" />
      </template>
      Find songs, artists and albums
      <span class="secondary block">All in one place.</span>
    </ScreenEmptyState>
  </ScreenBase>
</template>

<script lang="ts" setup>
import { intersectionBy } from 'lodash-es'
import { keepPreviousData, useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { queryClient } from '@/services/queryClient'
import { useSearchStore } from '@/stores/searchStore'
import { useRouter } from '@/composables/useRouter'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import PlayableExcerptResultsBlock from '@/components/screens/search/PlayableExcerptResultsBlock.vue'
import ArtistResultsBlock from '@/components/screens/search/ArtistExcerptResultsBlock.vue'
import AlbumResultsBlock from '@/components/screens/search/AlbumExcerptResultsBlock.vue'

const searchStore = useSearchStore()

const { url } = useRouter()

const q = ref('')

// Each search kept by its words: typing back to earlier ones shows them at once, and
// what was found shows while the next is looked for.
const { data, isFetching: searching } = useQuery({
  queryKey: computed(() => ['search', q.value]),
  queryFn: () => searchStore.excerptSearch(q.value),
  enabled: computed(() => q.value !== ''),
  placeholderData: keepPreviousData,
})

const excerpt = computed(() => data.value ?? { playables: [], albums: [], artists: [] })

const discoverUrl = computed(() => `${url('hunt')}?q=${encodeURIComponent(q.value)}`)

eventBus.on('SEARCH_KEYWORDS_CHANGED', keywords => (q.value = keywords))
eventBus.on('SONGS_DELETED', songs => {
  if (intersectionBy(songs, excerpt.value.playables, 'id').length !== 0) {
    queryClient.invalidateQueries({ queryKey: ['search'] })
  }
})
</script>
