<template>
  <section class="max-h-full min-h-full w-full flex flex-col transform-gpu overflow-hidden">
    <main ref="body" class="screen-body flex flex-col flex-1 place-content-start overflow-y-auto">
      <!-- In the scroller, so the header scrolls away; what must stay in reach is sticky. -->
      <div v-if="$slots.header" :class="{ tinted: tint }" class="screen-head">
        <slot name="header" />
      </div>
      <slot />
    </main>
  </section>
</template>

<script lang="ts" setup>
import { useEventListener } from '@vueuse/core'
import { onActivated, toRef, useTemplateRef } from 'vue'
import { useCoverTint } from '@/composables/useCoverTheme'

const props = withDefaults(
  defineProps<{
    /** A cover whose color tints the header softly (an album's, an artist's). */
    tintFrom?: string | null
  }>(),
  { tintFrom: null },
)

const tint = useCoverTint(toRef(props, 'tintFrom'))

const body = useTemplateRef('body')

// A screen kept alive while away loses its scroll position (the browser
// resets it when the screen leaves the page): it comes back where it was.
let scrollTop = 0
useEventListener(body, 'scroll', () => (scrollTop = body.value?.scrollTop ?? 0), { passive: true })

onActivated(() => {
  const el = body.value

  if (!el) {
    return
  }

  el.scrollTop = scrollTop
  // Virtual lists draw the rows for where the screen is now, even when that
  // didn't change.
  el.dispatchEvent(new Event('scroll'))
})
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

  @media (max-width: 768px), (max-height: 500px) and (pointer: coarse) {
    --screen-pad-x: 16px;
    --screen-pad-bottom: 16px;
  }

  .screen-head {
    flex-shrink: 0;
    margin: 0 calc(-1 * var(--screen-pad-x));

    &.tinted {
      background: linear-gradient(to bottom, color-mix(in srgb, v-bind(tint) 24%, transparent), transparent);
      transition: background 400ms linear;
    }
  }

  /* Lists and grids that run to the edges of the screen. */
  :slotted(.screen-bleed) {
    margin: 0 calc(-1 * var(--screen-pad-x)) calc(-1 * var(--screen-pad-bottom));
  }
}
</style>
