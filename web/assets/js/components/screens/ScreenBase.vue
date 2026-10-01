<template>
  <section class="max-h-full min-h-full w-full flex flex-col transform-gpu overflow-hidden">
    <main class="screen-body flex flex-col flex-1 place-content-start overflow-y-auto">
      <!-- In the scroller, so the header scrolls away; what must stay in reach is sticky. -->
      <div v-if="$slots.header" class="screen-head">
        <slot name="header" />
      </div>
      <HookSlot :context="{ screen: getCurrentScreen() }" name="screen.top" />
      <slot />
    </main>
  </section>
</template>

<script lang="ts" setup>
import { useRouter } from '@/composables/useRouter'

import HookSlot from '@/components/utils/HookSlot.vue'

const { getCurrentScreen } = useRouter()
</script>

<style lang="postcss" scoped>
main {
  -ms-overflow-style: -ms-autohiding-scrollbar;
}

.screen-body {
  --screen-pad-x: 24px;
  --screen-pad-bottom: 24px;

  position: relative;
  padding: 0 var(--screen-pad-x) var(--screen-pad-bottom);

  @media (max-width: 768px) {
    --screen-pad-x: 16px;
    --screen-pad-bottom: 16px;
  }

  .screen-head {
    flex-shrink: 0;
    margin: 0 calc(-1 * var(--screen-pad-x));
  }

  /* Lists and grids that run to the edges of the screen. */
  :slotted(.screen-bleed) {
    margin: 0 calc(-1 * var(--screen-pad-x)) calc(-1 * var(--screen-pad-bottom));
  }
}
</style>
