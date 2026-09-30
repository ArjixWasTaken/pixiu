<template>
  <div class="k-field k-select relative p-0!">
    <select ref="el" v-model="value" class="appearance-none w-full min-h-[46px] pl-4 pr-10 text-current">
      <slot />
    </select>
    <M3Icon
      class="pointer-events-none absolute top-1/2 -translate-y-1/2 right-2 text-(--schemes-on-surface-variant)"
      name="arrow_drop_down"
    />
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'

import M3Icon from '@/components/m3/M3Icon.vue'

const value = defineModel<any>()

const el = ref<HTMLInputElement>()

defineExpose({
  el,
})
</script>

<style lang="postcss" scoped>
select {
  background: none; /* remove background AND the dropdown arrow. Tailwind doesn't have a way to do this. */
}
</style>
