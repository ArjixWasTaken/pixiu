<template>
  <div :class="{ scrolls: scroll }" class="p-6">
    <slot />
  </div>
</template>

<script lang="ts" setup>
/**
 * In a dialog the panels scroll on their own (`scroll`); on a screen the
 * screen scrolls, so its scrollbar sits at the window's edge.
 */
withDefaults(defineProps<{ scroll?: boolean }>(), { scroll: true })
</script>

<style scoped>
.scrolls {
  overflow: auto;
}
</style>
