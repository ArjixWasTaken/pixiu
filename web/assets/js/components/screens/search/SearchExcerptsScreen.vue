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
import { computed, ref, toRef } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { searchStore } from '@/stores/searchStore'
import { useRouter } from '@/composables/useRouter'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import PlayableExcerptResultsBlock from '@/components/screens/search/PlayableExcerptResultsBlock.vue'
import ArtistResultsBlock from '@/components/screens/search/ArtistExcerptResultsBlock.vue'
import AlbumResultsBlock from '@/components/screens/search/AlbumExcerptResultsBlock.vue'

const { url } = useRouter()

const excerpt = toRef(searchStore.state, 'excerpt')
const q = ref('')
const searching = ref(false)

const discoverUrl = computed(() => `${url('hunt')}?q=${encodeURIComponent(q.value)}`)

const doSearch = async () => {
  searching.value = true
  await searchStore.excerptSearch(q.value)
  searching.value = false
}

eventBus.on('SEARCH_KEYWORDS_CHANGED', async _q => {
  q.value = _q
  await doSearch()
})
eventBus.on('SONGS_DELETED', async songs => {
  if (intersectionBy(songs, excerpt.value.playables, 'id').length !== 0) {
    await doSearch()
  }
})
</script>
