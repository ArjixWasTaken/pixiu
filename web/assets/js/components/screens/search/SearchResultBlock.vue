<template>
  <div class="pt-6">
    <header class="flex items-center justify-between gap-3 mb-3">
      <h3 class="m3-title-large text-(--schemes-on-surface) min-w-0">
        <slot name="header" />
      </h3>
      <div ref="actionsEl" class="flex items-center gap-2 shrink-0">
        <slot name="actions" />
      </div>
    </header>

    <slot />
  </div>
</template>

<script lang="ts" setup>
import { provide, useTemplateRef } from 'vue'
import { BlockActionsHostKey } from '@/config/symbols'

const actionsEl = useTemplateRef<HTMLElement>('actionsEl')
provide(BlockActionsHostKey, actionsEl)
</script>
