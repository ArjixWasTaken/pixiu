<template>
  <ScreenBase id="homeWrapper">
    <ScreenEmptyState v-if="libraryEmpty">
      <template #icon>
        <M3Icon name="volume_off" />
      </template>
      No songs found.
      <EmptyLibraryHint />
    </ScreenEmptyState>

    <div v-else class="home-sections flex flex-col gap-6 pt-6 w-full">
      <component v-for="block in shownBlocks" :key="block.id" :is="block.component" :loading :data-testid="block.id" />
      <div>
        <M3Button data-testid="reorder-home-blocks-btn" icon="tune" variant="text" @click="openReorderModal">
          Choose sections
        </M3Button>
      </div>
      <BtnScrollToTop />
    </div>
  </ScreenBase>
</template>

<script lang="ts" setup>
import type { Component } from 'vue'
import { computed, defineAsyncComponent, ref } from 'vue'
import { eventBus } from '@/utils/eventBus'
import { commonStore } from '@/stores/commonStore'
import { overviewStore } from '@/stores/overviewStore'
import { preferenceStore } from '@/stores/preferenceStore'
import { useRouter } from '@/composables/useRouter'
import { useModal } from '@/composables/useModal'
import { useErrorHandler } from '@/composables/useErrorHandler'

import MostPlayedSongs from '@/components/screens/home/MostPlayedSongs.vue'
import RecentlyPlayedPlayables from '@/components/screens/home/RecentlyPlayedPlayables.vue'
import NewAlbums from '@/components/screens/home/NewAlbums.vue'
import NewSongs from '@/components/screens/home/NewSongs.vue'
import TopArtists from '@/components/screens/home/TopArtists.vue'
import TopAlbums from '@/components/screens/home/TopAlbums.vue'
import NewArtists from '@/components/screens/home/NewArtists.vue'
import RandomAlbums from '@/components/screens/home/RandomAlbums.vue'
import RandomArtists from '@/components/screens/home/RandomArtists.vue'
import LeastPlayedSongs from '@/components/screens/home/LeastPlayedSongs.vue'
import RandomSongs from '@/components/screens/home/RandomSongs.vue'
import SimilarSongs from '@/components/screens/home/SimilarSongs.vue'
import M3Button from '@/components/m3/M3Button.vue'
import ScreenEmptyState from '@/components/ui/ScreenEmptyState.vue'
import BtnScrollToTop from '@/components/ui/BtnScrollToTop.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'
import EmptyLibraryHint from '@/components/ui/EmptyLibraryHint.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const ReorderBlocksModal = defineAsyncComponent(() => import('@/components/screens/home/ReorderBlocksModal.vue'))

interface Block {
  id: string
  label: string
  component: Component
}

const blocks: Block[] = [
  { id: 'recently-played-songs', label: 'Recently played', component: RecentlyPlayedPlayables },
  { id: 'recently-added-albums', label: 'Latest albums', component: NewAlbums },
  { id: 'similar-songs', label: 'You might also like', component: SimilarSongs },
  { id: 'most-played-albums', label: 'Top albums', component: TopAlbums },
  { id: 'most-played-songs', label: 'Most played', component: MostPlayedSongs },
  { id: 'most-played-artists', label: 'Top artists', component: TopArtists },
  { id: 'recently-added-songs', label: 'New songs', component: NewSongs },
  { id: 'recently-added-artists', label: 'New artists', component: NewArtists },
  { id: 'least-played-songs', label: 'Least played', component: LeastPlayedSongs },
  { id: 'random-songs', label: 'Random songs', component: RandomSongs },
  { id: 'random-albums', label: 'Random albums', component: RandomAlbums },
  { id: 'random-artists', label: 'Random artists', component: RandomArtists },
]

const { openModal } = useModal()

const libraryEmpty = computed(() => commonStore.state.song_length === 0)

const loading = ref(false)
let initialized = false

// Sort `blocks` so they appear in the order saved in the preference. Blocks
// whose id isn't in the saved list fall to the end (Infinity), keeping their
// canonical relative order via Array.sort's stability.
const orderedBlocks = computed<Block[]>(() => {
  const saved = preferenceStore.home_blocks_order ?? []
  const positionOf = (id: string) => {
    const i = saved.indexOf(id)
    return i === -1 ? Infinity : i
  }

  return [...blocks].sort((a, b) => positionOf(a.id) - positionOf(b.id))
})

/** The blocks shown: in order, less those switched off. */
const shownBlocks = computed(() =>
  orderedBlocks.value.filter(({ id }) => !(preferenceStore.home_blocks_hidden ?? []).includes(id)),
)

const openReorderModal = () =>
  openModal<'REORDER_HOME_BLOCKS'>(ReorderBlocksModal, {
    blocks: orderedBlocks.value.map(({ id, label }) => ({ id, label })),
  })

eventBus
  .on('SONGS_DELETED', () => overviewStore.fetch())
  .on('SONGS_UPDATED', () => overviewStore.fetch())
  .on('SONG_UPLOADED', () => overviewStore.fetch())

useRouter().onScreenActivated('Home', async () => {
  if (!initialized) {
    loading.value = true
    try {
      await overviewStore.fetch()
      initialized = true
    } catch (error: unknown) {
      useErrorHandler('dialog').handleHttpError(error)
    } finally {
      loading.value = false
    }
  }
})
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.home-sections {
  @apply min-w-0;

  > * {
    @apply min-w-0;
  }
}
</style>
