<template>
  <article
    v-if="!dismissed"
    class="fixed z-10000 left-4 flex items-center gap-3 max-w-xs py-3 px-4 rounded-xl border border-(--schemes-outline-variant) bg-(--schemes-surface-container) text-(--schemes-on-surface) shadow-lg cursor-pointer"
    title="Click to dismiss"
    @click="dismissed = true"
  >
    <M3Icon :size="18" class="shrink-0 text-(--schemes-tertiary)" name="wifi_off" />
    <span>You're offline.</span>
  </article>
</template>

<script lang="ts" setup>
import { ref, watch } from 'vue'
import { useNetworkStatus } from '@/composables/useNetworkStatus'

import M3Icon from '@/components/m3/M3Icon.vue'

const { online } = useNetworkStatus()
const dismissed = ref(false)

// Re-show the notification each time we go offline
watch(online, isOnline => {
  if (!isOnline) {
    dismissed.value = false
  }
})
</script>

<style lang="postcss" scoped>
article {
  bottom: calc(var(--footer-height) + 2rem);
}
</style>
