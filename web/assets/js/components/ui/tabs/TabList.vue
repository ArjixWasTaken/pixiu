<template>
  <nav class="tab-list flex overflow-x-auto px-6 min-h-[48px]" role="tablist">
    <slot />
  </nav>
</template>

<style scoped>
.tab-list {
  scrollbar-width: none;
  border-bottom: 1px solid var(--schemes-outline-variant);
}
</style>
