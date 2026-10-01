import type { Directive } from 'vue'

export const focus: Directive = {
  // On a field component (a label around its input), the input.
  mounted: (el: HTMLElement) => {
    const field = el.matches('input, textarea, select, button, [tabindex]')
      ? el
      : el.querySelector<HTMLElement>('input, textarea, select')
    ;(field ?? el).focus()
  },
}
