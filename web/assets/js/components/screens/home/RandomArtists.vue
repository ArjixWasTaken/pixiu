<template>
  <HomeScreenBlock v-if="loading || artists.length">
    <template #header>Random artists</template>
    <template #actions>
      <M3IconButton v-if="artists.length" :disabled="refreshing" label="Refresh" @click.prevent="refresh">
        <M3Icon :class="{ 'animate-spin': refreshing }" name="refresh" />
      </M3IconButton>
    </template>
    <Carousel>
      <template v-if="loading">
        <ArtistCardSkeleton v-for="i in 6" :key="i" />
      </template>
      <template v-else>
        <ArtistCard v-for="artist in artists" :key="artist.id" :artist />
      </template>
    </Carousel>
  </HomeScreenBlock>
</template>

<script lang="ts" setup>
import { ref, toRef, toRefs } from 'vue'
import { overviewStore } from '@/stores/overviewStore'
import { useErrorHandler } from '@/composables/useErrorHandler'

import ArtistCard from '@/components/artist/ArtistCard.vue'
import ArtistCardSkeleton from '@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'
import M3IconButton from '@/components/m3/M3IconButton.vue'
import Carousel from '@/components/ui/Carousel.vue'
import HomeScreenBlock from '@/components/screens/home/HomeScreenBlock.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const props = withDefaults(defineProps<{ loading?: boolean }>(), { loading: false })
const { loading } = toRefs(props)

const artists = toRef(overviewStore.state, 'randomArtists')
const refreshing = ref(false)

const refresh = async () => {
  refreshing.value = true

  try {
    await overviewStore.refreshRandomArtists()
  } catch (error: unknown) {
    useErrorHandler().handleHttpError(error)
  } finally {
    refreshing.value = false
  }
}
</script>
