<template>
  <span class="btn-group inline-flex relative flex-nowrap">
    <slot />
  </span>
</template>

<style lang="postcss" scoped>
.btn-group {
  gap: 8px;
  flex-wrap: wrap;
}
</style>
