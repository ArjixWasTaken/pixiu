<template>
  <article
    v-if="playable"
    class="fixed z-99 right-[5vw] top-18 flex bg-(--schemes-surface-container) border border-px border-(--schemes-outline-variant)"
  >
    <span :style="{ backgroundImage: `url(${defaultCover})` }">
      <img :src alt="Cover image" class="w-[96px] aspect-square object-cover" loading="lazy" />
    </span>
    <main class="px-5 py-4 min-w-80 max-w-96 flex flex-col justify-between overflow-hidden">
      <h4 class="uppercase">Up next</h4>
      <p class="text-(--schemes-on-surface) text-xl overflow-hidden whitespace-nowrap text-ellipsis">
        {{ playable.title }}
      </p>
      <p class="overflow-hidden whitespace-nowrap text-ellipsis">{{ author }}</p>
    </main>
  </article>
</template>

<script setup lang="ts">
import { computed, toRefs } from 'vue'
import { useBranding } from '@/composables/useBranding'
import { coverOfSize } from '@/services/subsonic'

const props = defineProps<{ playable: Playable }>()
const { playable } = toRefs(props)

const { cover: defaultCover } = useBranding()

const src = computed(() => coverOfSize(playable.value.album_cover, 128))
const author = computed(() => playable.value.artist_name)
</script>
