import { defineComponent, ref } from 'vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { usePlayableList } from '@/composables/usePlayableList'

describe('usePlayableList', () => {
  const h = createHarness()

  /** The list's songs as it would show them, sorted by `field`. */
  const sortedTitles = (titles: string[], field: PlayableListSortField) => {
    const songs = ref(titles.map(title => h.factory('song').make({ title }) as Playable))
    let list!: ReturnType<typeof usePlayableList>

    h.render(
      defineComponent({
        setup() {
          list = usePlayableList(songs, { type: 'Songs' })
          return () => null
        },
      }),
    )

    list.sort(field, 'asc')
    return list.filteredPlayables.value.map(({ title }) => title)
  }

  it('sorts text as people read it, without minding case', () => {
    expect(sortedTitles(['ANTHEM', 'Absolute Territory', 'ACID PHONK', 'a little messed up'], 'title')).toEqual([
      'a little messed up',
      'Absolute Territory',
      'ACID PHONK',
      'ANTHEM',
    ])
  })
})
