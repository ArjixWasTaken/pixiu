<template>
  <div class="now-playing-lyrics">
    <LyricsPane :song />
  </div>
</template>

<script lang="ts" setup>
import LyricsPane from '@/components/ui/lyrics/LyricsPane.vue'

defineProps<{ song: Song }>()
</script>

<style scoped>
.now-playing-lyrics {
  position: relative;
  height: 100%;
  min-height: 240px;
}
</style>
