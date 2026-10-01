<template>
  <HomeScreenBlock v-if="loading || playables.length">
    <template #header>Random songs</template>
    <template #actions>
      <M3IconButton v-if="playables.length" :disabled="refreshing" label="Refresh" @click.prevent="refresh">
        <M3Icon :class="{ 'animate-spin': refreshing }" name="refresh" />
      </M3IconButton>
    </template>
    <PlayableCardGridSkeleton v-if="loading" class="-mx-6" role="status" aria-busy="true" aria-label="Loading" />
    <template v-else>
      <PlayableCardGrid :aria-busy="refreshing" class="-mx-6" :playables />
    </template>
  </HomeScreenBlock>
</template>

<script lang="ts" setup>
import { ref, toRef, toRefs } from 'vue'
import { overviewStore } from '@/stores/overviewStore'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import HomeScreenBlock from '@/components/screens/home/HomeScreenBlock.vue'
import PlayableCardGrid from '@/components/screens/home/PlayableCardGrid.vue'
import PlayableCardGridSkeleton from '@/components/screens/home/PlayableCardGridSkeleton.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(defineProps<{ loading?: boolean }>(), { loading: false })
const { loading } = toRefs(props)

const playables = toRef(overviewStore.state, 'randomSongs')
const refreshing = ref(false)

const refresh = async () => {
  refreshing.value = true

  try {
    await overviewStore.refreshRandomSongs()
  } finally {
    refreshing.value = false
  }
}
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
:deep([aria-busy='true']) {
  @apply opacity-70 transition-opacity;
}
</style>
