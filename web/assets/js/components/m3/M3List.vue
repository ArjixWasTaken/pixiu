<template>
  <ul class="m3-list" role="list">
    <slot />
  </ul>
</template>

<style scoped>
.m3-list {
  display: flex;
  flex-direction: column;
  padding: 8px 0;
  margin: 0;
  list-style: none;
}
</style>
