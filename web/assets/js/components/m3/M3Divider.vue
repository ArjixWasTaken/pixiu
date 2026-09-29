<template>
  <hr :class="{ inset }" class="m3-divider" />
</template>

<script lang="ts" setup>
withDefaults(defineProps<{ inset?: boolean }>(), { inset: false })
</script>

<style scoped>
.m3-divider {
  border: 0;
  border-top: 1px solid var(--schemes-outline-variant);
  margin: 0;

  &.inset {
    margin: 0 16px;
  }
}
</style>
