<template>
  <div class="tabs">
    <header>
      <M3Tabs v-model="tab" :id-prefix :tabs />
    </header>
    <main>
      <slot />
    </main>
  </div>
</template>

<script lang="ts" setup>
import type { M3Tab } from '@/components/m3/M3Tabs.vue'
import M3Tabs from '@/components/m3/M3Tabs.vue'

/** An album's or artist's tabs, kept in reach while the screen scrolls; the panels are the slot's. */
defineProps<{ tabs: M3Tab[]; idPrefix: string }>()

const tab = defineModel<string>()
</script>

<style lang="postcss" scoped>
.tabs {
  display: flex;
  flex-direction: column;
  flex: 1;
  /* What sticks under the tabs (a list's toolbar) sticks this far down. */
  --sticky-top: calc(var(--m3-tab-height) + 1px);
}

/* The screen scrolls; the tabs stay in reach. */
header {
  position: sticky;
  top: 0;
  z-index: 6;
  flex-shrink: 0;
  padding: 0 24px;
  border-bottom: 1px solid var(--schemes-outline-variant);
  background: var(--schemes-surface);

  @media (max-width: 768px) {
    padding: 0 4px;
  }
}

:deep(.m3-tabs) {
  max-width: 560px;
}

main {
  display: flex;
  flex-direction: column;
  flex: 1;
}

:deep(:is(.songs-pane, .albums-pane)) {
  display: flex;
  flex-direction: column;
  flex: 1;
}

:deep(.albums-pane .none) {
  padding: 16px 24px;
}

:deep(.info-pane) {
  max-width: 72ch;
  padding: 20px 24px;
}
</style>
