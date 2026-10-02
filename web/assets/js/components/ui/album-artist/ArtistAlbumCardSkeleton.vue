<template>
  <!-- Shaped like the cards it stands in for: a square cover (a round picture for an artist), two lines. -->
  <article :class="{ round }" class="skeleton card">
    <span class="cover pulse" />
    <span class="line pulse" />
    <span class="line short pulse" />
  </article>
</template>

<script lang="ts" setup>
withDefaults(defineProps<{ round?: boolean }>(), { round: false })
</script>

<style scoped>
.card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 8px;
}

.cover {
  width: 100%;
  aspect-ratio: 1;
  border-radius: 12px;

  .round & {
    border-radius: 50%;
  }
}

.line {
  height: 12px;
  width: 75%;
  margin-left: 4px;
  border-radius: 6px;

  &.short {
    width: 45%;
  }
}
</style>
