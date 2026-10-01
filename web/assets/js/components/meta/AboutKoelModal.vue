<template>
  <div
    v-koel-focus
    class="about text-center max-w-[480px] overflow-hidden relative"
    data-testid="about-koel"
    tabindex="0"
    @keydown.esc="close"
  >
    <main class="p-6">
      <div class="mb-4">
        <img alt="Logo" class="inline-block" :src="logo" width="128" />
      </div>

      <div class="current-version">{{ appName }} {{ currentVersion }}</div>

      <p>
        A music server that downloads from YouTube Music, with a library for each of its users.
        <a href="https://github.com/ArjixWasTaken/pixiu" rel="noopener" target="_blank">Source code</a>
      </p>
    </main>

    <footer>
      <M3Button data-testid="close-modal-btn" variant="text" @click.prevent="close">Close</M3Button>
    </footer>
  </div>
</template>

<script lang="ts" setup>
import { useNewVersionNotification } from '@/composables/useNewVersionNotification'
import { useBranding } from '@/composables/useBranding'

import M3Button from '@/components/m3/M3Button.vue'

const emit = defineEmits<{ (e: 'close'): void }>()
const { name: appName, logo } = useBranding()
const { currentVersion } = useNewVersionNotification()

const close = () => emit('close')
</script>

<style lang="postcss" scoped>
@reference '@css/app.pcss';
p {
  @apply mx-0 my-3;
}

a {
  @apply text-(--schemes-on-surface) hover:text-(--schemes-primary);
}
</style>
