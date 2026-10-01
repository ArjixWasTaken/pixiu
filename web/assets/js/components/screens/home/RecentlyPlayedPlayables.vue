<template>
  <HomeScreenBlock v-if="loading || playables.length">
    <template #header>Recently played</template>
    <template #actions>
      <ViewAllRecentlyPlayedPlayablesButton v-if="playables.length" />
    </template>
    <PlayableCardGridSkeleton v-if="loading" class="-mx-6" role="status" aria-busy="true" aria-label="Loading" />
    <template v-else>
      <PlayableCardGrid class="-mx-6" :playables />
    </template>
  </HomeScreenBlock>
</template>

<script lang="ts" setup>
import { toRef, toRefs } from 'vue'
import { useOverviewStore } from '@/stores/overviewStore'
import HomeScreenBlock from '@/components/screens/home/HomeScreenBlock.vue'
import ViewAllRecentlyPlayedPlayablesButton from '@/components/screens/home/ViewAllRecentlyPlayedPlayablesButton.vue'
import PlayableCardGrid from '@/components/screens/home/PlayableCardGrid.vue'
import PlayableCardGridSkeleton from '@/components/screens/home/PlayableCardGridSkeleton.vue'

const overviewStore = useOverviewStore()

const props = withDefaults(defineProps<{ loading?: boolean }>(), { loading: false })
const { loading } = toRefs(props)

const playables = toRef(overviewStore.state, 'recentlyPlayed')
</script>
