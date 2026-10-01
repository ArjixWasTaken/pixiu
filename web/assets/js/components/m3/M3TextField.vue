<template>
  <label
    :class="[rootAttrs.class, { filled: hasValue, disabled, error, multiline }]"
    :style="rootAttrs.style"
    class="m3-text-field"
  >
    <span class="field">
      <M3Icon v-if="leadingIcon" :name="leadingIcon" class="icon" />
      <textarea
        v-if="multiline"
        v-model="value"
        :aria-label="label"
        v-bind="inputAttrs"
        :disabled
        :placeholder="placeholder || ' '"
        :rows
        class="m3-body-large"
      />
      <input
        v-else
        v-model="value"
        :aria-label="label"
        v-bind="inputAttrs"
        :disabled
        :placeholder="placeholder || ' '"
        :type
        class="m3-body-large"
      />
      <slot name="trailing">
        <M3Icon v-if="trailingIcon" :name="trailingIcon" class="icon" />
      </slot>
      <fieldset aria-hidden="true">
        <legend v-if="label" class="m3-body-small">
          <span>{{ label }}</span>
        </legend>
      </fieldset>
      <span v-if="label" :class="{ 'with-leading': leadingIcon }" class="label m3-body-large">{{ label }}</span>
    </span>
    <span v-if="supportingText" class="supporting m3-body-small">{{ supportingText }}</span>
  </label>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import { useSplitAttrs } from '@/components/m3/useSplitAttrs'
import M3Icon from '@/components/m3/M3Icon.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    label?: string
    placeholder?: string
    type?: string
    leadingIcon?: string
    trailingIcon?: string
    supportingText?: string
    disabled?: boolean
    error?: boolean
    multiline?: boolean
    rows?: number
  }>(),
  {
    type: 'text',
    disabled: false,
    error: false,
    multiline: false,
    rows: 3,
  },
)

const value = defineModel<string | number | null>({ default: '' })

const { rootAttrs, inputAttrs } = useSplitAttrs()

const hasValue = computed(() => (value.value !== null && String(value.value) !== '') || Boolean(props.placeholder))
</script>

<style scoped>
.m3-text-field {
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
  gap: 12px;
  min-height: var(--m3-field-height);
  padding: 0 16px;
}

.multiline .field {
  align-items: flex-start;
  padding-top: var(--m3-field-pad-y);
  padding-bottom: var(--m3-field-pad-y);
}

input,
textarea {
  flex: 1;
  min-width: 0;
  background: transparent;
  color: var(--schemes-on-surface);
  caret-color: var(--schemes-primary);
  border: 0;
  outline: 0;
  padding: 0;
  resize: vertical;

  /* A finger can't drag the corner: the field grows as it's typed in instead (field-sizing). */
  @media (pointer: coarse) {
    resize: none;
    field-sizing: content;
    min-height: 3lh;
  }

  &::placeholder {
    color: var(--schemes-on-surface-variant);
  }

  /* Numbers are typed, not stepped: no spin buttons. */
  &[type='number'] {
    appearance: textfield;

    &::-webkit-inner-spin-button,
    &::-webkit-outer-spin-button {
      appearance: none;
      margin: 0;
    }
  }
}

.icon {
  color: var(--schemes-on-surface-variant);
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
  max-width: 0.01px;
  height: 11px;
  padding: 0;
  white-space: nowrap;
  transition: max-width 50ms;

  span {
    padding: 0 4px;
  }
}

.label {
  position: absolute;
  left: 16px;
  top: 50%;
  transform: translateY(-50%);
  color: var(--schemes-on-surface-variant);
  pointer-events: none;
  transition:
    top 150ms var(--m3-ease),
    font-size 150ms var(--m3-ease),
    color 150ms linear;

  &.with-leading {
    left: 52px;
  }
}

.multiline .label {
  top: calc(var(--m3-field-pad-y) + var(--static-body-large-line-height) * 0.5px);
}

.m3-text-field:focus-within,
.filled {
  legend {
    max-width: 100%;
  }

  .label {
    top: 0;
    left: 16px;
    font-size: 12px;
    line-height: 16px;
  }
}

.m3-text-field:has(input:not(:placeholder-shown)),
.m3-text-field:has(textarea:not(:placeholder-shown)) {
  legend {
    max-width: 100%;
  }

  .label {
    top: 0;
    left: 16px;
    font-size: 12px;
    line-height: 16px;
  }
}

.m3-text-field:focus-within {
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

.error {
  fieldset {
    border-color: var(--schemes-error) !important;
  }

  .label,
  .supporting {
    color: var(--schemes-error) !important;
  }
}

.disabled {
  opacity: 0.38;
}
</style>
