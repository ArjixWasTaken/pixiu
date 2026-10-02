<template>
  <div
    v-koel-focus
    class="about text-center max-w-[480px] overflow-hidden relative"
    data-testid="about-koel"
    tabindex="0"
    @keydown.esc="close"
  >
    <!-- The dialog's header: room above the logo, as any dialog's title has. -->
    <header class="justify-center">
      <img :src="logo" alt="Logo" class="inline-block" width="128" />
    </header>

    <main>
      <div class="current-version m3-title-medium text-(--schemes-on-surface)">{{ appName }} {{ currentVersion }}</div>

      <p>
        A music server that downloads from YouTube Music and Deezer, with a library for each of its users.
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

/* A link, and it looks like one. */
a {
  color: var(--schemes-primary);
  text-decoration: underline;
  text-underline-offset: 2px;
}
</style>
