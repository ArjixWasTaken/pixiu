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
        A single-user music server that hunts YouTube Music.
        <a href="https://github.com/ArjixWasTaken/pixiu" rel="noopener" target="_blank">Source code</a>
      </p>
    </main>

    <footer>
      <Btn variant="destructive" data-testid="close-modal-btn" rounded @click.prevent="close">Close</Btn>
    </footer>
  </div>
</template>

<script lang="ts" setup>
import { useNewVersionNotification } from '@/composables/useNewVersionNotification'
import { useBranding } from '@/composables/useBranding'

import Btn from '@/components/ui/form/Btn.vue'

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
  @apply text-k-fg hover:text-k-highlight;
}
</style>
