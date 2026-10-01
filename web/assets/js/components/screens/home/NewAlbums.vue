<template>
  <HomeScreenBlock v-if="loading || albums.length">
    <template #header>Latest albums</template>
    <Carousel>
      <template v-if="loading">
        <AlbumCardSkeleton v-for="i in 6" :key="i" />
      </template>
      <template v-else>
        <AlbumCard v-for="album in albums" :key="album.id" :album />
      </template>
    </Carousel>
  </HomeScreenBlock>
</template>

<script lang="ts" setup>
import { toRef, toRefs } from 'vue'
import { overviewStore } from '@/stores/overviewStore'

import AlbumCard from '@/components/album/AlbumCard.vue'
import AlbumCardSkeleton from '@/components/ui/album-artist/ArtistAlbumCardSkeleton.vue'
import Carousel from '@/components/ui/Carousel.vue'
import HomeScreenBlock from '@/components/screens/home/HomeScreenBlock.vue'

const props = withDefaults(defineProps<{ loading?: boolean }>(), { loading: false })
const { loading } = toRefs(props)

const albums = toRef(overviewStore.state, 'recentlyAddedAlbums')
</script>
