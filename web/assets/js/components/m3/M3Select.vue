<template>
  <label :class="[rootAttrs.class, { disabled }]" :style="rootAttrs.style" class="m3-select">
    <span class="field">
      <select v-model="value" :aria-label="label" v-bind="inputAttrs" :disabled class="m3-body-large">
        <slot />
      </select>
      <M3Icon class="arrow" name="arrow_drop_down" />
      <fieldset aria-hidden="true">
        <legend v-if="label" class="m3-body-small">
          <span>{{ label }}</span>
        </legend>
      </fieldset>
      <span v-if="label" class="label m3-body-small">{{ label }}</span>
    </span>
    <span v-if="supportingText" class="supporting m3-body-small">{{ supportingText }}</span>
  </label>
</template>

<script lang="ts" setup>
import { useSplitAttrs } from '@/components/m3/useSplitAttrs'
import M3Icon from '@/components/m3/M3Icon.vue'

defineOptions({ inheritAttrs: false })

withDefaults(defineProps<{ label?: string; supportingText?: string; disabled?: boolean }>(), {
  disabled: false,
})

const value = defineModel<any>()

const { rootAttrs, inputAttrs } = useSplitAttrs()
</script>

<style scoped>
/* An outlined text field's shell around a native select; a select always
   shows a value, so its label always sits on the outline. */
.m3-select {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  color: var(--schemes-on-surface);
}

.field {
  position: relative;
  display: flex;
  align-items: center;
  min-height: var(--m3-field-height);
}

select {
  flex: 1;
  min-width: 0;
  height: var(--m3-field-height);
  padding: 0 40px 0 16px;
  appearance: none;
  background: transparent;
  color: var(--schemes-on-surface);
  border: 0;
  outline: 0;
  cursor: pointer;
  text-overflow: ellipsis;

  & :deep(option) {
    background: var(--schemes-surface-container);
    color: var(--schemes-on-surface);
  }
}

.arrow {
  position: absolute;
  right: 12px;
  color: var(--schemes-on-surface-variant);
  pointer-events: none;
}

fieldset {
  position: absolute;
  inset: -5px 0 0;
  margin: 0;
  padding: 0 12px;
  border: 1px solid var(--schemes-outline);
  border-radius: 4px;
  pointer-events: none;
  text-align: start;
}

legend {
  visibility: hidden;
  max-width: 100%;
  height: 11px;
  padding: 0;
  white-space: nowrap;

  span {
    padding: 0 4px;
  }
}

.label {
  position: absolute;
  top: 0;
  left: 16px;
  transform: translateY(-50%);
  font-size: 12px;
  line-height: 16px;
  color: var(--schemes-on-surface-variant);
  pointer-events: none;
}

.m3-select:focus-within {
  fieldset {
    border: 2px solid var(--schemes-primary);
  }

  .label {
    color: var(--schemes-primary);
  }
}

.supporting {
  padding: 0 16px;
  color: var(--schemes-on-surface-variant);
}

.disabled {
  opacity: 0.38;
}
</style>
