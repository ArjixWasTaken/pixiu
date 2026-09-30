<template>
  <label :class="[rootAttrs.class, { checked }]" :style="rootAttrs.style" class="m3-checkbox m3-state">
    <input v-model="checked" v-bind="inputAttrs" type="checkbox" />
    <span class="box">
      <M3Icon v-if="checked" :size="18" name="check" />
    </span>
  </label>
</template>

<script lang="ts" setup>
import M3Icon from '@/components/m3/M3Icon.vue'
import { useSplitAttrs } from '@/components/m3/useSplitAttrs'

defineOptions({ inheritAttrs: false })

const checked = defineModel<boolean>({ default: false })

const { rootAttrs, inputAttrs } = useSplitAttrs()
</script>

<style scoped>
.m3-checkbox {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  flex-shrink: 0;
  border-radius: 50%;
  color: var(--schemes-on-surface-variant);
  cursor: pointer;

  &.checked {
    color: var(--schemes-primary);
  }
}

input {
  position: absolute;
  inset: 0;
  margin: 0;
  opacity: 0;
  cursor: pointer;
}

.box {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border: 2px solid var(--schemes-on-surface-variant);
  border-radius: 2px;
  box-sizing: border-box;
  pointer-events: none;
}

.checked .box {
  border-color: var(--schemes-primary);
  background: var(--schemes-primary);
  color: var(--schemes-on-primary);
}
</style>
