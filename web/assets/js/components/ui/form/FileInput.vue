<template>
  <label
    class="inline-flex text-base items-center gap-2 relative cursor-pointer hover:text-(--schemes-on-surface-variant) active:text-(--schemes-on-surface-variant)"
  >
    <input v-bind="$attrs" type="file" class="opacity-0 appearance-none absolute inset-0 z-10" />
    <span class="border border-px border-(--schemes-outline-variant) px-1.5 py-1 rounded-sm pointer-events-none">
      <M3Icon name="attach_file" />
    </span>
    <span class="text-1 pointer-events-none">
      <slot>Select a file…</slot>
    </span>
  </label>
</template>

<script setup lang="ts">
import M3Icon from '@/components/m3/M3Icon.vue'

defineOptions({ inheritAttrs: false })
</script>
