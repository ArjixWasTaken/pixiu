<template>
  <header class="flex gap-2 items-center">
    <template v-if="!saveDialogOpen">
      <M3Select :model-value="selectedId" label="Preset" @update:model-value="id => emit('select', id)">
        <option :value="null" disabled>Preset</option>
        <template v-if="customPresets.length">
          <optgroup label="Built-in">
            <option v-for="preset in builtInPresets" :key="preset.id ?? ''" :value="preset.id ?? ''">
              {{ preset.name }}
            </option>
          </optgroup>
          <optgroup label="Custom">
            <option v-for="preset in customPresets" :key="preset.id ?? ''" :value="preset.id ?? ''">
              {{ preset.name }}
            </option>
          </optgroup>
        </template>
        <option v-for="preset in builtInPresets" v-else :key="preset.id ?? ''" :value="preset.id ?? ''">
          {{ preset.name }}
        </option>
      </M3Select>

      <M3Button v-if="isModified" variant="text" @click.prevent="saveDialogOpen = true">Save as…</M3Button>
      <M3Button v-if="customSelected" variant="text" @click.prevent="emit('delete')">Delete</M3Button>
    </template>

    <EqualizerSavePresetForm v-else @submit="commitSave" @cancel="saveDialogOpen = false" />
  </header>
</template>

<script lang="ts" setup>
import { ref, toRef } from 'vue'
import { equalizerPresets as builtInPresets } from '@/config/audio'
import { useEqualizerStore } from '@/stores/equalizerStore'
import M3Button from '@/components/m3/M3Button.vue'
import M3Select from '@/components/m3/M3Select.vue'
import EqualizerSavePresetForm from '@/components/ui/equalizer/EqualizerSavePresetForm.vue'

const equalizerStore = useEqualizerStore()

defineProps<{
  selectedId: string | null
  isModified: boolean
  customSelected: boolean
}>()

const emit = defineEmits<{
  (e: 'select', id: string | null): void
  (e: 'save', name: string): void
  (e: 'delete'): void
}>()

const customPresets = toRef(equalizerStore.state, 'customPresets')
const saveDialogOpen = ref(false)

const commitSave = (name: string) => {
  emit('save', name)
  saveDialogOpen.value = false
}
</script>
