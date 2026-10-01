<template>
  <ScreenBase>
    <template #header>
      <ScreenHeader :disabled="loading" :layout="playables.length ? headerLayout : 'collapsed'">
        Results for “{{ q }}”

        <template #thumbnail>
          <ThumbnailStack :thumbnails="thumbnails" />
        </template>

        <template v-if="playables.length" #meta>
          <span>{{ songCount }}</span>
          <span>{{ duration }}</span>
        </template>

        <template #controls>
          <PlayableListControls v-if="playables.length" :config @play-all="playAll" @play-selected="playSelected" />
        </template>
      </ScreenHeader>
    </template>

    <PlayableListSkeleton v-if="loading" class="screen-bleed" role="status" aria-busy="true" aria-label="Loading" />
    <PlayableList v-else ref="playableList" class="screen-bleed" @press:enter="onPressEnter" @swipe="onSwipe" />
  </ScreenBase>
</template>

<script lang="ts" setup>
import { ref, toRef } from 'vue'
import { useSearchStore } from '@/stores/searchStore'
import { usePlayableList } from '@/composables/usePlayableList'
import { usePlayableListControls } from '@/composables/usePlayableListControls'
import { useRouter } from '@/composables/useRouter'

import ScreenHeader from '@/components/ui/ScreenHeader.vue'
import PlayableListSkeleton from '@/components/playable/playable-list/PlayableListSkeleton.vue'
import ScreenBase from '@/components/screens/ScreenBase.vue'

const searchStore = useSearchStore()

const { getRouteParam, onScreenActivated } = useRouter()
const q = ref('')

const {
  PlayableList,
  ThumbnailStack,
  headerLayout,
  playables,
  playableList,
  thumbnails,
  duration,
  onPressEnter,
  playAll,
  playSelected,
  onSwipe,
  songCount,
} = usePlayableList(toRef(searchStore.state, 'playables'), { type: 'Search.Playables' })

const { PlayableListControls, config } = usePlayableListControls('Search.Playables')
const loading = ref(false)

// Kept alive between visits: each visit looks for its own words.
onScreenActivated('Search.Playables', async () => {
  const words = getRouteParam('q') || ''

  if (words === q.value && playables.value.length) {
    return
  }

  q.value = words
  searchStore.resetPlayableResultState()

  if (!words) {
    return
  }

  loading.value = true
  await searchStore.playableSearch(words)
  loading.value = false
})
</script>
