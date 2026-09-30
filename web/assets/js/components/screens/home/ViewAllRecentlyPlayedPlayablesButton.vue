<template>
  <M3Button variant="text" @click.prevent="goToRecentlyPlayedScreen">View all</M3Button>
</template>

<script lang="ts" setup>
import { useRouter } from '@/composables/useRouter'

import M3Button from '@/components/m3/M3Button.vue'

const { go, url } = useRouter()
const goToRecentlyPlayedScreen = () => go(url('recently-played'))
</script>
