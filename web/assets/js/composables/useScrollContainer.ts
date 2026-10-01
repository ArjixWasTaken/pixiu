import { useEventListener, useResizeObserver } from '@vueuse/core'
import { onMounted, ref, shallowRef, useTemplateRef } from 'vue'

/**
 * What scrolls a virtual list (its element is the `list` template ref). On a
 * screen, the screen does (`.screen-body`), so the screen's header scrolls away
 * above it; anywhere else, the list scrolls itself.
 */
export const useScrollContainer = () => {
  const list = useTemplateRef<HTMLElement>('list')
  const scroller = shallowRef<HTMLElement | null>(null)
  /** Whether the screen scrolls the list, rather than the list itself. */
  const nested = ref(false)
  /** Where the list starts within what scrolls it. */
  const margin = ref(0)
  /** The list's own width. */
  const width = ref(0)

  const measure = () => {
    if (!list.value || !scroller.value) {
      return
    }

    margin.value = nested.value
      ? list.value.getBoundingClientRect().top - scroller.value.getBoundingClientRect().top + scroller.value.scrollTop
      : 0
    width.value = list.value.clientWidth
  }

  onMounted(() => {
    scroller.value = list.value!.parentElement?.closest<HTMLElement>('.screen-body') ?? list.value!
    nested.value = scroller.value !== list.value
    measure()
  })

  // What is above the list can change height as the screen scrolls (its header shrinks).
  useEventListener(scroller, 'scroll', measure, { passive: true })
  useResizeObserver(list, measure)

  return { list, scroller, nested, margin, width }
}
