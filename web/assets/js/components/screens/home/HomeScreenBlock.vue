<template>
  <section class="min-w-0" data-vue="HomeScreenBlock">
    <header class="flex items-center justify-between gap-3 mb-3">
      <h2 class="m3-title-large text-(--schemes-on-surface) min-w-0">
        <slot name="header" />
      </h2>
      <div ref="actionsEl" class="flex items-center gap-1 shrink-0">
        <slot name="actions" />
      </div>
    </header>

    <slot />
  </section>
</template>

<script lang="ts" setup>
import { provide, useTemplateRef } from 'vue'
import { BlockActionsHostKey } from '@/config/symbols'

const actionsEl = useTemplateRef<HTMLElement>('actionsEl')
provide(BlockActionsHostKey, actionsEl)
</script>
