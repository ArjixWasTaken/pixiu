<template>
  <component
    :is="tag"
    :class="{ 'm3-state interactive': interactive, selected, 'two-line': supporting || $slots.supporting }"
    class="m3-list-item"
  >
    <div v-if="$slots.leading" class="leading">
      <slot name="leading" />
    </div>
    <div class="content">
      <p v-if="overline" class="m3-label-medium overline-text">{{ overline }}</p>
      <p class="m3-body-large headline">
        <slot name="headline">{{ headline }}</slot>
      </p>
      <p v-if="supporting || $slots.supporting" class="m3-body-medium supporting">
        <slot name="supporting">{{ supporting }}</slot>
      </p>
    </div>
    <div v-if="$slots.trailing" class="trailing m3-label-small">
      <slot name="trailing" />
    </div>
  </component>
</template>

<script lang="ts" setup>
withDefaults(
  defineProps<{
    headline?: string
    supporting?: string
    overline?: string
    interactive?: boolean
    selected?: boolean
    tag?: string
  }>(),
  {
    interactive: false,
    selected: false,
    tag: 'li',
  },
)
</script>

<style scoped>
.m3-list-item {
  display: flex;
  align-items: center;
  gap: 16px;
  min-height: var(--m3-list-item-height);
  padding: var(--m3-list-item-pad-y) 24px var(--m3-list-item-pad-y) 16px;
  color: var(--schemes-on-surface);
  list-style: none;
  text-align: start;

  &.two-line {
    min-height: var(--m3-list-item-height-2);
  }

  &.interactive {
    cursor: pointer;
  }

  &.selected {
    background: var(--schemes-secondary-container);
    color: var(--schemes-on-secondary-container);
  }
}

.leading,
.trailing {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  gap: 4px;
  color: var(--schemes-on-surface-variant);
}

.content {
  flex: 1;
  min-width: 0;
}

.headline,
.supporting,
.overline-text {
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.supporting,
.overline-text {
  color: var(--schemes-on-surface-variant);
}
</style>
