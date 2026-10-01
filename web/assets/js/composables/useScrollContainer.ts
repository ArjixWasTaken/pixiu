import { useEventListener, useMutationObserver, useResizeObserver } from '@vueuse/core'
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, useTemplateRef } from 'vue'

const isHTMLElement = (el: Element): el is HTMLElement => el instanceof HTMLElement

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
  /** The screen's parts (header, panels, the list's own…), whose sizes move the list. */
  const screenParts = shallowRef<HTMLElement[]>([])

  const measure = () => {
    if (!list.value || !scroller.value) {
      return
    }

    margin.value = nested.value
      ? list.value.getBoundingClientRect().top - scroller.value.getBoundingClientRect().top + scroller.value.scrollTop
      : 0
    width.value = list.value.clientWidth
  }

  // On the next frame: changing the list's size within the observer's own round is a loop.
  let frame = 0
  const measureSoon = () => {
    cancelAnimationFrame(frame)
    frame = requestAnimationFrame(measure)
  }
  onBeforeUnmount(() => cancelAnimationFrame(frame))

  const collectScreenParts = () => {
    screenParts.value = nested.value && scroller.value ? [...scroller.value.children].filter(isHTMLElement) : []
  }

  onMounted(() => {
    scroller.value = list.value!.parentElement?.closest<HTMLElement>('.screen-body') ?? list.value!
    nested.value = scroller.value !== list.value
    collectScreenParts()
    measure()
  })

  // What is above the list can change height as the screen scrolls (its header shrinks).
  useEventListener(scroller, 'scroll', measure, { passive: true })

  useResizeObserver(list, measureSoon)

  // On a screen, what comes before the list can grow, shrink, appear or go (a
  // loading placeholder, a panel opening): the list moves without changing size.
  useResizeObserver(screenParts, measureSoon)
  useMutationObserver(
    computed(() => (nested.value ? scroller.value : null)),
    () => {
      collectScreenParts()
      measureSoon()
    },
    { childList: true },
  )

  return { list, scroller, nested, margin, width }
}
