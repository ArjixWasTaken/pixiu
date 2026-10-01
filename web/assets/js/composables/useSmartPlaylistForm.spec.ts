import { describe, expect, it } from 'vite-plus/test'
import { nextTick } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { playlistStore } from '@/stores/playlistStore'
import { useSmartPlaylistForm } from './useSmartPlaylistForm'

describe('useSmartPlaylistForm', () => {
  createHarness()

  const form = (html: string) => {
    const element = document.createElement('form')
    element.innerHTML = html
    document.body.append(element)
    return element
  }

  it('starts on the details tab, with a copy of the groups it was given', () => {
    const group = playlistStore.createEmptySmartPlaylistRuleGroup()
    const { tabs, currentTab, ruleGroups, rulesChanged } = useSmartPlaylistForm([group])

    expect(tabs.map(tab => tab.id)).toEqual(['details', 'rules'])
    expect(currentTab.value).toBe('details')
    expect(ruleGroups.value).toEqual([group])
    expect(rulesChanged()).toBe(false)

    ruleGroups.value[0].rules[0].value = ['changed']
    expect(group.rules[0].value).toEqual([''])
    expect(rulesChanged()).toBe(true)
  })

  it('shows the hidden tab holding a blank field', async () => {
    const { currentTab, onInvalid } = useSmartPlaylistForm([])
    const element = form(
      '<div data-tab="details"><input name="name" value="Mine"></div><div data-tab="rules"><input required name="value"></div>',
    )
    element.addEventListener('invalid', onInvalid, true)

    element.querySelector<HTMLInputElement>('[name="value"]')!.dispatchEvent(new Event('invalid'))
    await nextTick()

    expect(currentTab.value).toBe('rules')
    element.remove()
  })

  it('leaves a blank field on the tab shown to the browser', () => {
    const { currentTab, onInvalid } = useSmartPlaylistForm([])
    const element = form(
      '<div data-tab="details"><input required name="name"></div><div data-tab="rules"><input required name="value"></div>',
    )
    element.addEventListener('invalid', onInvalid, true)

    element.querySelector<HTMLInputElement>('[name="value"]')!.dispatchEvent(new Event('invalid'))

    expect(currentTab.value).toBe('details')
    element.remove()
  })
})
