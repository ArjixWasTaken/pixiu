import { useMediaQuery } from '@vueuse/core'
import { computed, ref } from 'vue'

/**
 * Phones: up to 768px wide, or held sideways (short, and touched). Stylesheets
 * use the same condition (`@media (max-width: 768px), (max-height: 500px) and
 * (pointer: coarse)`), so a phone in landscape keeps the phone's layout.
 */
export const PHONE_QUERY = '(max-width: 768px), (max-height: 500px) and (pointer: coarse)'

/**
 * The design's breakpoints: phones (see above), wide song grids from 1360px;
 * and touch, for a finger rather than a mouse (no dragging, tap to play,
 * transcoding for phone networks).
 */
const media = {
  mobile: useMediaQuery(PHONE_QUERY),
  wide: useMediaQuery('(min-width: 1360px)'),
  touch: useMediaQuery('(pointer: coarse)'),
}

type Viewport = Record<keyof typeof media, boolean>

const pretend = ref<Partial<Viewport>>({})

const isMobile = computed(() => pretend.value.mobile ?? media.mobile.value)
const isWide = computed(() => pretend.value.wide ?? media.wide.value)
const isTouch = computed(() => pretend.value.touch ?? media.touch.value)

export const useViewport = () => ({ isMobile, isWide, isTouch })

/** For specs: pretend to be on a phone or a desktop, and to have a mouse unless told otherwise. */
export const setViewport = ({
  mobile,
  wide = false,
  touch = false,
}: {
  mobile: boolean
  wide?: boolean
  touch?: boolean
}) => (pretend.value = { mobile, wide, touch })
