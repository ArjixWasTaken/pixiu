<template>
  <ol class="playable-card-grid">
    <PlayableCard v-for="playable in playables" :key="playable.id" :playable />
  </ol>
</template>

<script lang="ts" setup>
import { toRefs } from 'vue'

import PlayableCard from '@/components/screens/home/PlayableCard.vue'

const props = defineProps<{ playables: Playable[] }>()
const { playables } = toRefs(props)
</script>

<style scoped>
.playable-card-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 0 16px;
  margin: 0 -12px;

  @media (min-width: 1360px) {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
