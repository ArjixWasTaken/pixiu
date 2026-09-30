import { readonly, ref } from 'vue'

/** The design's breakpoints: phones up to 768px wide, wide song grids from 1360px. */
const MOBILE = '(max-width: 768px)'
const WIDE = '(min-width: 1360px)'

const isMobile = ref(false)
const isWide = ref(false)
let listening = false

const listen = () => {
  if (listening || typeof window === 'undefined' || !window.matchMedia) {
    return
  }

  listening = true

  for (const [query, target] of [
    [MOBILE, isMobile],
    [WIDE, isWide],
  ] as const) {
    const list = window.matchMedia(query)
    target.value = list.matches
    list.addEventListener?.('change', event => (target.value = event.matches))
  }
}

export const useViewport = () => {
  listen()

  return {
    isMobile: readonly(isMobile),
    isWide: readonly(isWide),
  }
}

/** For specs: pretend to be on a phone or a desktop. */
export const setViewport = ({ mobile, wide = false }: { mobile: boolean; wide?: boolean }) => {
  listen()
  isMobile.value = mobile
  isWide.value = wide
}
