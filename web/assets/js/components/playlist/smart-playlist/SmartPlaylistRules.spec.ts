import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { usePlaylistStore } from '@/stores/playlistStore'
import Component from './SmartPlaylistRules.vue'

describe('smartPlaylistRules', () => {
  const h = createHarness()

  const renderComponent = (groups: SmartPlaylistRuleGroup[]) => h.render(Component, { props: { modelValue: groups } })

  const updated = (emitted: Record<string, unknown[][]>) =>
    emitted['update:modelValue'].at(-1)![0] as SmartPlaylistRuleGroup[]

  it('adds a group with a rule to fill in', async () => {
    const { emitted } = renderComponent([])

    await h.user.click(screen.getByRole('button', { name: /Add a group/ }))

    const groups = updated(emitted())
    expect(groups).toHaveLength(1)
    expect(groups[0].rules).toHaveLength(1)
  })

  it('offers another group next to the ones there are', async () => {
    const { emitted } = renderComponent([usePlaylistStore().createEmptySmartPlaylistRuleGroup()])

    await h.user.click(screen.getByRole('button', { name: /Add another group/ }))

    expect(updated(emitted())).toHaveLength(2)
  })

  it('drops a group with its last rule', async () => {
    const { emitted } = renderComponent([usePlaylistStore().createEmptySmartPlaylistRuleGroup()])

    await h.user.click(screen.getByRole('button', { name: 'Remove this rule' }))

    expect(updated(emitted())).toEqual([])
  })
})
