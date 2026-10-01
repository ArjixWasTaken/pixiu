<template>
  <HomeScreenBlock v-if="loading || artists.length">
    <template #header>Top artists</template>
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
import { toRef, toRefs } from 'vue'
import { useOverviewStore } from '@/stores/overviewStore'
import ArtistCard from '@/components/artist/ArtistCard.vue'
import ArtistCardSkeleton from '@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'
import Carousel from '@/components/ui/Carousel.vue'
import HomeScreenBlock from '@/components/screens/home/HomeScreenBlock.vue'

const overviewStore = useOverviewStore()

const props = withDefaults(defineProps<{ loading?: boolean }>(), { loading: false })
const { loading } = toRefs(props)

const artists = toRef(overviewStore.state, 'mostPlayedArtists')
</script>
