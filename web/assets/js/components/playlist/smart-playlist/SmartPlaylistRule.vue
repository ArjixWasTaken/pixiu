<template>
  <div class="rule-container" data-testid="smart-playlist-rule">
    <div class="rule">
      <M3Select :model-value="rule.model.name" class="model" label="Field" @update:model-value="setModel">
        <option v-for="option in models" :key="option.name" :value="option.name">{{ option.label }}</option>
      </M3Select>

      <M3Select :model-value="operator.operator" class="condition" label="Condition" @update:model-value="setOperator">
        <option v-for="option in operators" :key="option.operator" :value="option.operator">{{ option.label }}</option>
      </M3Select>

      <div class="values">
        <M3TextField
          v-for="(value, index) in values"
          :key="index"
          :label="index === 0 ? 'Value' : 'And'"
          :model-value="value"
          :type="inputType"
          class="value"
          name="value[]"
          required
          @update:model-value="setValue(index, $event)"
        >
          <template v-if="unit" #trailing>
            <span class="unit m3-body-medium">{{ unit }}</span>
          </template>
        </M3TextField>
      </div>

      <M3IconButton class="remove" icon="close" label="Remove this rule" @click="emit('remove')" />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed } from 'vue'
import models from '@/config/smart-playlist/models'
import inputTypes from '@/config/smart-playlist/inputTypes'

import M3IconButton from '@/components/m3/M3IconButton.vue'
import M3Select from '@/components/m3/M3Select.vue'
import M3TextField from '@/components/m3/M3TextField.vue'

const props = defineProps<{ rule: SmartPlaylistRule }>()

const emit = defineEmits<{
  (e: 'update:rule', rule: SmartPlaylistRule): void
  (e: 'remove'): void
}>()

const operatorsFor = (model: SmartPlaylistModel) => inputTypes[model.type]

const operators = computed(() => operatorsFor(props.rule.model))

/** The rule's operator; the first there is when the model has no such one. */
const operator = computed(
  () => operators.value.find(({ operator }) => operator === props.rule.operator) ?? operators.value[0],
)

const inputCount = (operator: SmartPlaylistOperator) => operator.inputs ?? 1
const typeOf = (model: SmartPlaylistModel, operator: SmartPlaylistOperator) => operator.type ?? model.type

const inputType = computed(() => typeOf(props.rule.model, operator.value))
const unit = computed(() => operator.value.unit ?? props.rule.model.unit)

const values = computed(() =>
  Array.from({ length: inputCount(operator.value) }, (_, index) => props.rule.value[index] ?? ''),
)

const update = (changes: Partial<SmartPlaylistRule>) => emit('update:rule', { ...props.rule, ...changes })

/** Another field keeps the condition and value when they still fit it. */
const setModel = (name: SmartPlaylistModel['name']) => {
  const model = models.find(option => option.name === name)!
  if (model.type === props.rule.model.type) {
    update({ model })
    return
  }
  const first = operatorsFor(model)[0]
  update({ model, operator: first.operator, value: Array.from({ length: inputCount(first) }, () => '') })
}

/** Another condition keeps the value when it takes the same kind. */
const setOperator = (name: SmartPlaylistOperator['operator']) => {
  const next = operators.value.find(({ operator }) => operator === name)!
  const keeps =
    inputCount(next) === inputCount(operator.value) &&
    typeOf(props.rule.model, next) === typeOf(props.rule.model, operator.value)
  update({
    operator: next.operator,
    value: keeps ? [...values.value] : Array.from({ length: inputCount(next) }, () => ''),
  })
}

const setValue = (index: number, value: string | number | null) => {
  const next = [...values.value]
  next[index] = value ?? ''
  update({ value: next })
}
</script>

<style scoped>
.rule-container {
  container-type: inline-size;
}

.rule {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1.5fr) auto;
  grid-template-areas: 'model condition values remove';
  align-items: start;
  gap: 8px;
}

/* Narrow: the values go under the field and condition. */
@container (max-width: 520px) {
  .rule {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) auto;
    grid-template-areas:
      'model condition remove'
      'values values .';
  }
}

.model {
  grid-area: model;
}

.condition {
  grid-area: condition;
}

.values {
  grid-area: values;
  display: flex;
  gap: 8px;
  min-width: 0;
}

.value {
  flex: 1 1 0;
}

.unit {
  color: var(--schemes-on-surface-variant);
}

.remove {
  grid-area: remove;
  margin-top: 8px;
}
</style>
