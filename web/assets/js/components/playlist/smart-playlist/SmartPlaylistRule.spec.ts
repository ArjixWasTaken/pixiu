import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import models from '@/config/smart-playlist/models'
import Component from './SmartPlaylistRule.vue'

describe('smartPlaylistRule', () => {
  const h = createHarness()

  const model = (name: SmartPlaylistModel['name']) => models.find(m => m.name === name)!

  const createRule = (overrides: Partial<SmartPlaylistRule> = {}): SmartPlaylistRule => ({
    id: 'rule-1',
    model: model('title'),
    operator: 'is',
    value: [''],
    ...overrides,
  })

  const renderComponent = (rule = createRule()) => h.render(Component, { props: { rule } })

  const options = (label: string) =>
    Array.from(screen.getByRole('combobox', { name: label }).querySelectorAll('option')).map(
      option => option.textContent,
    )

  /** What the rule became, from its last update. */
  const updated = (emitted: Record<string, unknown[][]>) => emitted['update:rule'].at(-1)![0] as SmartPlaylistRule

  it('labels its field, condition and value', () => {
    renderComponent()

    expect(options('Field')).toEqual(models.map(m => m.label))
    screen.getByRole('textbox', { name: 'Value' })
  })

  it.each<[SmartPlaylistModel['name'], string[]]>([
    ['title', ['is', 'is not', 'contains', 'does not contain', 'begins with', 'ends with']],
    ['year', ['is', 'is not', 'is greater than', 'is less than', 'is between']],
    ['interactions.last_played_at', ['is', 'is not', 'in the last', 'not in the last', 'is between']],
  ])('offers the conditions of %s', (name, conditions) => {
    renderComponent(createRule({ model: model(name) }))

    expect(options('Condition')).toEqual(conditions)
  })

  it('takes two values for “is between”', () => {
    renderComponent(createRule({ model: model('year'), operator: 'isBetween', value: ['2000', '2020'] }))

    expect(screen.getAllByRole('spinbutton').map(input => (input as HTMLInputElement).value)).toEqual(['2000', '2020'])
    screen.getByRole('spinbutton', { name: 'And' })
  })

  it.each<[SmartPlaylistRule['operator'], SmartPlaylistModel['name'], string]>([
    ['inLast', 'interactions.last_played_at', 'days'],
    ['is', 'length', 'seconds'],
  ])('names the unit for %s %s', (operator, name, unit) => {
    renderComponent(createRule({ model: model(name), operator, value: ['7'] }))

    screen.getByText(unit)
  })

  it('updates with what is typed', async () => {
    const { emitted } = renderComponent()

    await h.user.type(screen.getByRole('textbox', { name: 'Value' }), 'x')

    expect(updated(emitted())).toMatchObject({ id: 'rule-1', operator: 'is', value: ['x'] })
  })

  it('keeps the condition and value for a field of the same kind', async () => {
    const { emitted } = renderComponent(createRule({ operator: 'contains', value: ['love'] }))

    await h.user.selectOptions(screen.getByRole('combobox', { name: 'Field' }), 'album.name')

    expect(updated(emitted())).toMatchObject({ model: model('album.name'), operator: 'contains', value: ['love'] })
  })

  it('starts over for a field of another kind', async () => {
    const { emitted } = renderComponent(createRule({ operator: 'contains', value: ['love'] }))

    await h.user.selectOptions(screen.getByRole('combobox', { name: 'Field' }), 'year')

    expect(updated(emitted())).toMatchObject({ model: model('year'), operator: 'is', value: [''] })
  })

  it('makes room for a second value for “is between”', async () => {
    const { emitted } = renderComponent(createRule({ model: model('year'), value: ['1999'] }))

    await h.user.selectOptions(screen.getByRole('combobox', { name: 'Condition' }), 'isBetween')

    expect(updated(emitted())).toMatchObject({ operator: 'isBetween', value: ['', ''] })
  })

  it('is removed on request', async () => {
    const { emitted } = renderComponent()

    await h.user.click(screen.getByRole('button', { name: 'Remove this rule' }))

    expect(emitted().remove).toBeTruthy()
  })
})
