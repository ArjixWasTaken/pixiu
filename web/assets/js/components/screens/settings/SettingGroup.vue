<template>
  <M3Card class="setting-group" variant="outlined">
    <header v-if="$slots.title || $slots.subtitle">
      <h3 v-if="$slots.title" class="m3-title-large text-(--schemes-on-surface)">
        <slot name="title" />
      </h3>
      <p v-if="$slots.subtitle" class="m3-body-medium mt-1 text-(--schemes-on-surface-variant)">
        <slot name="subtitle" />
      </p>
    </header>

    <div class="m3-body-medium">
      <slot />
    </div>

    <footer v-if="$slots.footer">
      <slot name="footer" />
    </footer>
  </M3Card>
</template>

<script lang="ts" setup>
import M3Card from '@/components/m3/M3Card.vue'
</script>

<style scoped>
.setting-group {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 24px;

  @media (max-width: 768px) {
    padding: 16px;
  }
}
</style>
