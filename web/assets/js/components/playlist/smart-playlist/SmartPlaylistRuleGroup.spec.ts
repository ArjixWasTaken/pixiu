import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import models from '@/config/smart-playlist/models'
import Component from './SmartPlaylistRuleGroup.vue'

describe('smartPlaylistRuleGroup', () => {
  const h = createHarness()

  const rule = (id: string): SmartPlaylistRule => ({ id, model: models[0], operator: 'is', value: [id] })

  const renderComponent = (rules = [rule('a')], isFirstGroup = true) =>
    h.render(Component, { props: { group: { id: 'group-1', rules }, isFirstGroup } })

  const updated = (emitted: Record<string, unknown[][]>) => emitted['update:group'].at(-1)![0] as SmartPlaylistRuleGroup

  it.each([
    [true, 'Songs that match all of these'],
    [false, 'Or songs that match all of these'],
  ])('says what its rules mean (first: %s)', (first, heading) => {
    renderComponent(undefined, first)

    screen.getByRole('heading', { name: heading })
  })

  it('shows each rule', () => {
    renderComponent([rule('a'), rule('b')])

    expect(screen.getAllByTestId('smart-playlist-rule')).toHaveLength(2)
  })

  it('adds a rule', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: /Add a rule/ }))

    expect(updated(emitted()).rules).toHaveLength(2)
  })

  it('drops a removed rule', async () => {
    const { emitted } = renderComponent([rule('a'), rule('b')])

    await h.user.click(screen.getAllByRole('button', { name: 'Remove this rule' })[0])

    expect(updated(emitted()).rules.map(({ id }) => id)).toEqual(['b'])
  })

  it('goes with its last rule', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Remove this rule' }))

    expect(emitted().remove).toBeTruthy()
    expect(emitted()['update:group']).toBeUndefined()
  })
})
