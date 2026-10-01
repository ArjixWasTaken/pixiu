<template>
  <span class="inline-flex shrink-0 opacity-50!" title="Available offline">
    <M3Icon :size="16" name="cloud_download" />
  </span>
</template>

<script lang="ts" setup>
/** Marks a song kept for offline playback; sized and aligned as the caching and error marks it replaces. */
import M3Icon from '@/components/m3/M3Icon.vue'
</script>
