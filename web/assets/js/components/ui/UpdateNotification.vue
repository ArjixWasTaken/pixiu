<template>
  <article
    v-if="newerVersionDeployed"
    data-testid="update-notification"
    class="fixed z-10000 left-4 flex items-center gap-3 py-3 pl-4 pr-3 rounded-xl border border-(--schemes-outline-variant) bg-(--schemes-surface-container) text-(--schemes-on-surface) shadow-lg"
  >
    <M3Icon name="rocket_launch" :size="18" class="shrink-0 text-(--schemes-on-surface-variant)" />
    <span class="whitespace-nowrap">{{ appName }} has been updated.</span>
    <M3Button @click="forceReloadWindow">Reload</M3Button>
  </article>
</template>

<script lang="ts" setup>
import { onBeforeUnmount, ref } from 'vue'
import { useEventListener } from '@vueuse/core'
import { forceReloadWindow } from '@/utils/helpers'
import { useBranding } from '@/composables/useBranding'
import { isNewerVersionDeployed } from '@/utils/deployment'
import { eventBus } from '@/utils/eventBus'

import M3Button from '@/components/m3/M3Button.vue'
import M3Icon from '@/components/m3/M3Icon.vue'

const { name: appName } = useBranding()
const newerVersionDeployed = ref(false)

const showNotice = () => {
  newerVersionDeployed.value = true
}

eventBus.on('NEW_VERSION_DEPLOYED', showNotice)
onBeforeUnmount(() => eventBus.off('NEW_VERSION_DEPLOYED', showNotice))

useEventListener(window, 'vite:preloadError', async () => {
  if (await isNewerVersionDeployed()) {
    showNotice()
  }
})
</script>

<style lang="postcss" scoped>
article {
  bottom: calc(var(--footer-height) + 2rem);
}
</style>
