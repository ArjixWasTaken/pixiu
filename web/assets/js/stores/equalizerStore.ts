import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { equalizerPresets as builtInPresets } from '@/config/audio'
import { uuid } from '@/utils/crypto'

const byName = (a: EqualizerPreset, b: EqualizerPreset) =>
  (a.name ?? '').localeCompare(b.name ?? '', undefined, { sensitivity: 'base' })

export const useEqualizerStore = defineStore('equalizer', () => {
  const preferences = usePreferenceStore()

  const state = reactive({
    customPresets: [] as EqualizerPreset[],
  })

  const init = () => {
    state.customPresets = [...(preferences.equalizer_presets ?? [])].sort(byName)
  }

  const isModified = (preset: any) =>
    typeof preset === 'object' &&
    preset !== null &&
    !preset.id &&
    preset.name === null &&
    typeof preset.preamp === 'number' &&
    Array.isArray(preset.gains) &&
    preset.gains.length === 10 &&
    preset.gains.every((gain: any) => typeof gain === 'number')

  const getPresetById = (id: string) =>
    builtInPresets.find(p => p.id === id) ?? state.customPresets.find(p => p.id === id)

  const getConfig = (): EqualizerPreset => {
    const current = preferences.current_equalizer_preset

    if (current.id) {
      // If the saved preset was deleted elsewhere, keep the user's slider
      // state by demoting to a modified preset.
      return getPresetById(current.id) ?? { name: null, preamp: current.preamp, gains: [...current.gains] }
    }

    // Backwards-compat: legacy data persisted name without id.
    if (current.name !== null) {
      return builtInPresets.find(p => p.name === current.name) ?? builtInPresets[0]
    }

    return current
  }

  const saveConfig = (preset: EqualizerPreset | null, preamp: number, gains: number[]) => {
    preferences.current_equalizer_preset = preset ?? { name: null, preamp, gains }
  }

  // Custom presets are kept with the other preferences, in the browser.
  const saveCustomPreset = async (name: string, preamp: number, gains: number[]): Promise<EqualizerPreset> => {
    const preset: EqualizerPreset = { id: uuid(), name, preamp, gains: [...gains] }
    state.customPresets = [...state.customPresets, preset].sort(byName)
    preferences.equalizer_presets = state.customPresets

    return preset
  }

  const deleteCustomPreset = async (id: string) => {
    state.customPresets = state.customPresets.filter(p => p.id !== id)
    preferences.equalizer_presets = state.customPresets
  }

  return { state, init, isModified, getPresetById, getConfig, saveConfig, saveCustomPreset, deleteCustomPreset }
})
