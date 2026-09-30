import { isEqual } from 'lodash-es'
import { nextTick, ref, toRaw } from 'vue'
import type { M3Tab } from '@/components/m3/M3Tabs.vue'

type SmartPlaylistFormTab = 'details' | 'rules'

const tabs: M3Tab[] = [
  { id: 'details', label: 'Details' },
  { id: 'rules', label: 'Rules' },
]

/**
 * What the smart playlist forms share: their tabs, and the rule groups being
 * edited, starting from `initialRuleGroups`.
 */
export const useSmartPlaylistForm = (initialRuleGroups: SmartPlaylistRuleGroup[]) => {
  const initial = structuredClone(toRaw(initialRuleGroups))

  const currentTab = ref<SmartPlaylistFormTab>('details')
  const ruleGroups = ref<SmartPlaylistRuleGroup[]>(structuredClone(initial))

  const rulesChanged = () => !isEqual(toRaw(ruleGroups.value), initial)

  let reporting = false

  /**
   * A field left blank on the hidden tab (a rule's value, say) stops the
   * form without a word: show that tab and the field. A blank field on the
   * tab shown is the browser's to point out.
   */
  const onInvalid = (event: Event) => {
    const form = event.currentTarget as HTMLFormElement
    const field = event.target as HTMLInputElement
    const tab = field.closest<HTMLElement>('[data-tab]')?.dataset.tab as SmartPlaylistFormTab | undefined

    if (reporting || !tab || tab === currentTab.value) {
      return
    }

    const shown = form.querySelector(`[data-tab="${currentTab.value}"]`)
    const fields = shown?.querySelectorAll<HTMLInputElement>('input, select, textarea') ?? []
    if (Array.from(fields).some(other => !other.validity.valid)) {
      return
    }

    reporting = true
    currentTab.value = tab
    nextTick(() => {
      field.reportValidity()
      reporting = false
    })
  }

  return {
    tabs,
    currentTab,
    ruleGroups,
    rulesChanged,
    onInvalid,
  }
}
