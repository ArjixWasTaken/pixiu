<template>
  <div class="select-none w-full flex flex-col" tabindex="0" @keydown.esc="close">
    <div class="title-row">
      <h1 class="m3-headline-small">Equalizer</h1>
      <label class="flex items-center gap-2 m3-label-large">
        {{ enabled ? 'On' : 'Off' }}
        <M3Switch v-model="enabled" aria-label="Equalizer on" name="equalizer_enabled" />
      </label>
    </div>

    <EqualizerHeader
      :selected-id="selectedId"
      :is-modified="isModified"
      :custom-selected="customSelected"
      @select="applySelection"
      @save="commitSave"
      @delete="confirmDelete"
    />

    <main>
      <EqualizerBands
        ref="bandsRef"
        :bands="bands"
        :class="{ off: !enabled }"
        @user-change="onUserChange"
        @commit="save"
      />
    </main>

    <footer class="border-t-(--schemes-outline-variant)">
      <M3Button @click.prevent="close">Close</M3Button>
    </footer>
  </div>
</template>

<script lang="ts" setup>
import { computed, onMounted, ref } from 'vue'
import { equalizerStore } from '@/stores/equalizerStore'
import { audioService } from '@/services/audioService'
import { equalizerPresets as builtInPresets } from '@/config/audio'
import { useDialogBox } from '@/composables/useDialogBox'
import { preferenceStore } from '@/stores/preferenceStore'

import M3Button from '@/components/m3/M3Button.vue'
import M3Switch from '@/components/m3/M3Switch.vue'
import EqualizerBands from '@/components/ui/equalizer/EqualizerBands.vue'
import EqualizerHeader from '@/components/ui/equalizer/EqualizerHeader.vue'

const emit = defineEmits<{ (e: 'close'): void }>()

const { showConfirmDialog } = useDialogBox()

const bands = audioService.bands

/** On, or off: audio plays past it, and the settings stay for later. */
const enabled = computed({
  get: () => preferenceStore.equalizer_enabled !== false,
  set: on => {
    preferenceStore.equalizer_enabled = on
    audioService.setBypassed(!on)
  },
})
const selectedId = ref<string | null>(null)
const bandsRef = ref<InstanceType<typeof EqualizerBands>>()

const isModified = computed(() => selectedId.value === null)

const customSelected = computed(
  () => selectedId.value !== null && !builtInPresets.some(preset => preset.id === selectedId.value),
)

const save = () =>
  equalizerStore.saveConfig(
    selectedId.value === null ? null : (equalizerStore.getPresetById(selectedId.value) ?? null),
    bandsRef.value?.getPreamp() ?? 0,
    bands.map(band => band.db),
  )

const applySelection = async (id: string | null) => {
  selectedId.value = id

  if (id !== null) {
    await bandsRef.value?.loadPreset(equalizerStore.getPresetById(id) ?? builtInPresets[0], bands)
  }

  save()
}

const onUserChange = () => {
  selectedId.value = null
}

const commitSave = async (name: string) => {
  const created = await equalizerStore.saveCustomPreset(
    name,
    bandsRef.value?.getPreamp() ?? 0,
    bands.map(band => band.db),
  )

  selectedId.value = created.id ?? null
  save()
}

const confirmDelete = async () => {
  if (!customSelected.value || selectedId.value === null) {
    return
  }

  if (!(await showConfirmDialog('Delete this preset?'))) {
    return
  }

  await equalizerStore.deleteCustomPreset(selectedId.value)
  selectedId.value = null
  save()
}

const close = () => emit('close')

onMounted(async () => {
  equalizerStore.init()
  const preset = equalizerStore.getConfig()
  await bandsRef.value?.loadPreset(preset, bands)
  selectedId.value = preset.id ?? null
})
</script>

<style scoped>
/* Padded like the dialog's header, which follows with the presets. */
.title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 24px 24px 0;
  color: var(--schemes-on-surface);
}

/* Off: the bands stay, for when it is back on. */
.off {
  opacity: 0.5;
}
</style>
