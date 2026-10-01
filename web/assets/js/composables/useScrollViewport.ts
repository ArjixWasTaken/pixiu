import type { Ref } from 'vue'
import { onBeforeUnmount, onMounted, ref } from 'vue'

/**
 * How far a virtual list has scrolled, and how much of it shows. A list on a
 * screen scrolls with the screen (`.screen-body`), so the screen's header
 * scrolls away above it; anywhere else the list scrolls itself.
 */
export const useScrollViewport = (list: Ref<HTMLElement | undefined>, onScrolled?: () => void) => {
  /** How much of the list has scrolled out of view at the top. */
  const scrollTop = ref(0)
  /** The height of what shows of the scroller. */
  const height = ref(0)
  /** The list's own width. */
  const width = ref(0)
  /** Whether the screen scrolls the list, rather than the list itself. */
  const nested = ref(false)

  let scroller: HTMLElement | undefined
  let frame = 0

  /** Where the list starts within the scroller's content. */
  const offset = () => {
    if (!scroller || !list.value || !nested.value) {
      return 0
    }

    return list.value.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop
  }

  const measure = () => {
    if (!scroller || !list.value) {
      return
    }

    scrollTop.value = Math.max(0, scroller.scrollTop - offset())
    height.value = scroller.clientHeight
    width.value = list.value.clientWidth
  }

  const onScroll = () => {
    cancelAnimationFrame(frame)

    frame = requestAnimationFrame(() => {
      measure()
      onScrolled?.()
    })
  }

  /** Whether the scroller is within `slack` px of its end. */
  const nearEnd = (slack: number) =>
    Boolean(scroller) && scroller!.scrollTop + scroller!.clientHeight + slack >= scroller!.scrollHeight

  /** Scrolls to `top` px into the list. */
  const scrollTo = (top: number, behavior: ScrollBehavior = 'smooth') =>
    scroller?.scrollTo({ top: Math.max(0, offset() + top), behavior })

  const resizeObserver = new ResizeObserver(() => measure())

  onMounted(() => {
    scroller = list.value!.parentElement?.closest<HTMLElement>('.screen-body') ?? list.value!
    nested.value = scroller !== list.value

    scroller.addEventListener('scroll', onScroll, { passive: true })
    resizeObserver.observe(scroller)
    nested.value && resizeObserver.observe(list.value!)
    measure()
  })

  onBeforeUnmount(() => {
    cancelAnimationFrame(frame)
    scroller?.removeEventListener('scroll', onScroll)
    resizeObserver.disconnect()
  })

  return { scrollTop, height, width, nested, nearEnd, scrollTo }
}
