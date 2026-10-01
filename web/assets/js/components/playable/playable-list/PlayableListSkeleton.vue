<template>
  <!-- Shaped like the rows it stands in for, so nothing jumps when they come. -->
  <div class="skeleton">
    <div v-for="key in 12" :key class="row">
      <div class="cover pulse" />
      <div class="lines">
        <div :style="{ width: `${titleWidths[key % titleWidths.length]}%` }" class="line pulse" />
        <div :style="{ width: `${titleWidths[(key + 3) % titleWidths.length] / 2}%` }" class="line small pulse" />
      </div>
      <div class="time pulse" />
    </div>
  </div>
</template>

<script setup lang="ts">
/** Varied, so the placeholders read as a list of different songs. */
const titleWidths = [42, 64, 36, 55, 48, 70, 40]
</script>

<style lang="postcss" scoped>
.skeleton {
  padding: 0 12px;
}

.row {
  display: flex;
  align-items: center;
  gap: 16px;
  height: var(--m3-row-height);
  padding: 0 8px 0 calc(var(--screen-pad-x, 24px) - 12px);
}

.cover {
  width: var(--m3-row-cover);
  height: var(--m3-row-cover);
  flex-shrink: 0;
  border-radius: 6px;
}

.lines {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.line {
  height: 12px;
  border-radius: 6px;

  &.small {
    height: 10px;
  }
}

.time {
  width: 36px;
  height: 10px;
  margin-right: 88px;
  border-radius: 5px;

  @media (max-width: 768px) {
    display: none;
  }
}
</style>
