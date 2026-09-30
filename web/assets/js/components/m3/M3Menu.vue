<template>
  <div class="m3-menu" role="menu">
    <slot />
  </div>
</template>

<style scoped>
.m3-menu {
  display: flex;
  flex-direction: column;
  min-width: 112px;
  max-width: 320px;
  padding: 8px 0;
  border-radius: 4px;
  background: var(--schemes-surface-container);
  color: var(--schemes-on-surface);
  box-shadow: var(--m3-elevation-2);
}
</style>
