<template>
  <h3 class="header m3-title-small text-(--schemes-on-surface-variant)">
    <slot />
  </h3>
</template>

<style scoped>
.header {
  padding: 18px 16px;

  @media (pointer: fine) and (min-width: 769px) {
    padding: 16px 12px 6px;
  }
}
</style>
