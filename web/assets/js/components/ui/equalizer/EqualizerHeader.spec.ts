import { describe, expect, it } from 'vite-plus/test'
import { fireEvent, screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { equalizerPresets as builtInPresets } from '@/config/audio'
import { useEqualizerStore } from '@/stores/equalizerStore'
import Component from './EqualizerHeader.vue'

const rock = builtInPresets.find(preset => preset.name === 'Rock')!
const customPreset: EqualizerPreset = { id: '01HFCUSTOM01', name: 'My Bass', preamp: 3, gains: [4, 4, 0] }

describe('equalizerHeader.vue', () => {
  const h = createHarness({
    beforeEach: () => {
      useEqualizerStore().state.customPresets = [customPreset]
    },
  })

  const renderHeader = (
    overrides: Partial<{
      selectedId: string | null
      isModified: boolean
      customSelected: boolean
    }> = {},
  ) =>
    h.render(Component, {
      props: {
        isModified: overrides.isModified ?? false,
        customSelected: overrides.customSelected ?? false,
        selectedId: overrides.selectedId ?? builtInPresets[0].id ?? null,
      },
    })

  it('renders built-in and custom presets in grouped optgroups', () => {
    renderHeader()

    screen.getByText('Default')
    screen.getByText('Rock')
    screen.getByText('My Bass')
  })

  it('omits the Custom optgroup when there are no custom presets', () => {
    useEqualizerStore().state.customPresets = []
    renderHeader()

    expect(screen.queryByText('My Bass')).toBeNull()
  })

  it('hides Save as… when not modified', () => {
    renderHeader({ isModified: false })
    expect(screen.queryByText('Save as…')).toBeNull()
  })

  it('shows Save as… when modified', () => {
    renderHeader({ isModified: true, selectedId: null })
    screen.getByText('Save as…')
  })

  it('hides Delete when no custom preset is selected', () => {
    renderHeader({ customSelected: false })
    expect(screen.queryByText('Delete')).toBeNull()
  })

  it('shows Delete when a custom preset is selected', () => {
    renderHeader({ customSelected: true, selectedId: '01HFCUSTOM01' })
    screen.getByText('Delete')
  })

  it('emits delete when Delete is clicked', async () => {
    const { emitted } = renderHeader({ customSelected: true, selectedId: '01HFCUSTOM01' })

    await fireEvent.click(screen.getByText('Delete'))

    expect(emitted().delete).toHaveLength(1)
  })

  it('opens the save form when Save as… is clicked, then closes it on cancel', async () => {
    renderHeader({ isModified: true, selectedId: null })

    await fireEvent.click(screen.getByText('Save as…'))
    screen.getByRole('textbox', { name: 'Preset name' })

    await fireEvent.click(screen.getByText('Cancel'))
    expect(screen.queryByRole('textbox', { name: 'Preset name' })).toBeNull()
  })

  it('emits save with the entered name when the form is submitted, and closes the dialog', async () => {
    const { emitted } = renderHeader({ isModified: true, selectedId: null })

    await fireEvent.click(screen.getByText('Save as…'))

    const input = screen.getByRole<HTMLInputElement>('textbox', { name: 'Preset name' })
    await fireEvent.update(input, 'Bass Boost')
    await fireEvent.click(screen.getByText('Save'))

    expect(emitted().save).toEqual([['Bass Boost']])
    expect(screen.queryByRole('textbox', { name: 'Preset name' })).toBeNull()
  })

  it('emits select with the new id when a preset is chosen', async () => {
    const { emitted } = renderHeader()

    await h.user.selectOptions(screen.getByRole('combobox'), rock.id ?? '')

    expect(emitted().select).toEqual([[rock.id]])
  })
})
