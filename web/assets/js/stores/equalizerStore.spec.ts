import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { usePreferenceStore } from '@/stores/preferenceStore'
import { useEqualizerStore } from '@/stores/equalizerStore'
describe('equalizerStore', () => {
  createHarness({
    beforeEach: () => {
      usePreferenceStore().current_equalizer_preset = {
        id: '01KR9JKWWQDDJZ5HT6DBY9DH3Y',
        name: 'Default',
        preamp: 0,
        gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      }
      usePreferenceStore().equalizer_presets = []
      useEqualizerStore().init()
    },
  })

  it('finds a built-in preset by id', () => {
    const preset = useEqualizerStore().getPresetById('01KR9JKWWQDDJZ5HT6DBY9DH49')
    expect(preset).toBeDefined()
    expect(preset!.name).toBe('Rock')
  })

  it('finds a custom preset by id', () => {
    const custom: EqualizerPreset = { id: '01HCUSTOM', name: 'Mine', preamp: 0, gains: [] }
    useEqualizerStore().state.customPresets = [custom]
    expect(useEqualizerStore().getPresetById('01HCUSTOM')).toEqual(custom)
  })

  it('returns undefined for an unknown id', () => {
    expect(useEqualizerStore().getPresetById('does-not-exist')).toBeUndefined()
  })

  it('returns built-in config from preferences by id', () => {
    usePreferenceStore().current_equalizer_preset = {
      id: '01KR9JKWWQDDJZ5HT6DBY9DH3Z',
      name: 'Classical',
      preamp: 0,
      gains: [],
    }
    expect(useEqualizerStore().getConfig().name).toBe('Classical')
  })

  it('falls back to legacy name lookup when no id is persisted', () => {
    usePreferenceStore().current_equalizer_preset = { name: 'Classical', preamp: 0, gains: [] }
    expect(useEqualizerStore().getConfig().name).toBe('Classical')
  })

  it('returns Default preset when neither id nor name resolves', () => {
    usePreferenceStore().current_equalizer_preset = { name: 'DoesNotExist', preamp: 0, gains: [] }
    expect(useEqualizerStore().getConfig().name).toBe('Default')
  })

  it('returns modified preset directly when name is null', () => {
    usePreferenceStore().current_equalizer_preset = {
      name: null,
      preamp: 5,
      gains: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
    }
    const config = useEqualizerStore().getConfig()
    expect(config.name).toBeNull()
    expect(config.preamp).toBe(5)
  })

  it('isModified is true only when id and name are both falsy', () => {
    expect(useEqualizerStore().isModified({ name: null, preamp: 3, gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })).toBe(true)
    expect(useEqualizerStore().isModified({ name: 'Rock', preamp: 0, gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })).toBe(
      false,
    )
    expect(
      useEqualizerStore().isModified({ id: '01J0', name: null, preamp: 0, gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }),
    ).toBe(false)
  })

  it('saves last-applied config (named preset)', () => {
    const rock = useEqualizerStore().getPresetById('01KR9JKWWQDDJZ5HT6DBY9DH49')!
    useEqualizerStore().saveConfig(rock, 0, rock.gains)
    expect(usePreferenceStore().current_equalizer_preset.name).toBe('Rock')
    expect(usePreferenceStore().current_equalizer_preset.id).toBe('01KR9JKWWQDDJZ5HT6DBY9DH49')
  })

  it('saves last-applied config (modified)', () => {
    useEqualizerStore().saveConfig(null, 7, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    expect(usePreferenceStore().current_equalizer_preset.name).toBeNull()
    expect(usePreferenceStore().current_equalizer_preset.preamp).toBe(7)
  })
})
