<template>
  <ExcerptResultBlock>
    <template #header>
      Songs

      <M3Button
        v-if="playables.length && !searching"
        data-testid="view-all-songs-btn"
        variant="text"
        @click.prevent="goToSongResults"
      >
        View all
      </M3Button>
    </template>

    <PlayableListSkeleton v-if="searching" role="status" aria-busy="true" aria-label="Loading" />
    <template v-else>
      <PlayableList v-if="displayedPlayables.length" ref="playableList" class="-mx-3" @press:enter="onPressEnter" />
      <p v-else class="m3-body-medium text-(--schemes-on-surface-variant)">Nothing found.</p>
    </template>
  </ExcerptResultBlock>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'
import { useRouter } from '@/composables/useRouter'
import { usePlayableList } from '@/composables/usePlayableList'
import { playback } from '@/services/playbackManager'

import ExcerptResultBlock from '@/components/screens/search/ExcerptResultBlock.vue'
import M3Button from '@/components/m3/M3Button.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'

const props = withDefaults(defineProps<{ playables?: Playable[]; query?: string; searching?: boolean }>(), {
  playables: () => [],
  query: '',
  searching: false,
})

const { playables, query, searching } = toRefs(props)

const {
  PlayableList,
  playables: displayedPlayables,
  playableList,
  selectedPlayables,
} = usePlayableList(
  playables,
  {},
  {
    sortable: false,
  },
)

const { go, url } = useRouter()

const onPressEnter = () => selectedPlayables.value.length && playback().play(selectedPlayables.value[0])
const goToSongResults = () => go(`${url('search.playables')}/?q=${encodeURIComponent(query.value)}`)
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
.results {
  @apply grid grid-cols-1 md:grid-cols-2 gap-x-4 gap-y-3;
}
</style>
