import { fireEvent, screen } from '@testing-library/vue'
import { describe, expect, it, vi } from 'vite-plus/test'
import { nextTick } from 'vue'
import { createHarness } from '@/__tests__/TestHarness'
import { usePreferenceStore } from '@/stores/preferenceStore'
import Component from './ReorderBlocksModal.vue'

const blocks = [
  { id: 'recently-played-songs', label: 'Recently played' },
  { id: 'recently-added-albums', label: 'Latest albums' },
  { id: 'most-played-albums', label: 'Top albums' },
  { id: 'random-songs', label: 'Random songs' },
]

const dispatch = (target: EventTarget, type: string, init: Record<string, unknown> = {}) => {
  const event = new Event(type, { bubbles: true, cancelable: true })
  for (const [key, value] of Object.entries(init)) {
    Object.defineProperty(event, key, { value, configurable: true })
  }
  target.dispatchEvent(event)
  return event
}

const stubRect = (el: HTMLElement, top: number, height = 40) => {
  el.getBoundingClientRect = () =>
    ({
      top,
      bottom: top + height,
      height,
      left: 0,
      right: 300,
      width: 300,
      x: 0,
      y: top,
      toJSON: () => ({}),
    }) as DOMRect
}

const rowIds = (container: Element) =>
  Array.from(container.querySelectorAll<HTMLElement>('[draggable="true"] > .flex-1')).map(
    el => el.textContent?.trim() ?? '',
  )

describe('ReorderBlocksModal', () => {
  const h = createHarness({
    beforeEach: () => {
      usePreferenceStore().home_blocks_order = []
      usePreferenceStore().home_blocks_hidden = []
    },
  })

  it('renders one row per block in canonical order when no preference is set', () => {
    h.render(Component, { props: { blocks } })

    blocks.forEach(b => screen.getByText(b.label))
  })

  it('renders rows in the order they arrive via props (the parent owns the sort)', () => {
    const reordered = [blocks[3], blocks[1], blocks[2], blocks[0]]

    const { container } = h.render(Component, { props: { blocks: reordered } })
    const labels = rowIds(container)

    expect(labels).toEqual(['Random songs', 'Latest albums', 'Top albums', 'Recently played'])
  })

  it('marks the dragged row with the opacity-40 modifier while a drag is in flight', async () => {
    const { container } = h.render(Component, { props: { blocks } })
    const rows = container.querySelectorAll<HTMLElement>('[draggable="true"]')

    dispatch(rows[0], 'dragstart', { dataTransfer: { effectAllowed: '' } })
    await nextTick()

    expect(rows[0].classList.contains('opacity-40')).toBe(true)
  })

  it('reorders the rendered rows reactively as the cursor hovers over a target', async () => {
    const { container } = h.render(Component, { props: { blocks } })
    const rows = Array.from(container.querySelectorAll<HTMLElement>('[draggable="true"]'))

    // Source at row 0 (Recently played), target at row 2 (Top albums).
    stubRect(rows[0], 0)
    stubRect(rows[2], 80)

    dispatch(rows[0], 'dragstart', { dataTransfer: { effectAllowed: '' } })
    // Lower half of the target → insert after.
    dispatch(rows[2], 'dragover', { clientY: 105 })
    await nextTick()

    // After the reorder, the source should sit after Top Albums in the DOM.
    const labels = rowIds(container)
    const sourceIdx = labels.indexOf('Recently played')
    const targetIdx = labels.indexOf('Top albums')
    expect(sourceIdx).toBeGreaterThan(targetIdx)
  })

  it('persists the current order to usePreferenceStore().home_blocks_order on dragend', async () => {
    const { container } = h.render(Component, { props: { blocks } })
    const rows = Array.from(container.querySelectorAll<HTMLElement>('[draggable="true"]'))

    stubRect(rows[0], 0)
    stubRect(rows[2], 80)

    dispatch(rows[0], 'dragstart', { dataTransfer: { effectAllowed: '' } })
    dispatch(rows[2], 'dragover', { clientY: 105 })
    await nextTick()
    dispatch(rows[0], 'dragend')

    const saved = usePreferenceStore().home_blocks_order
    const sourceIdx = saved.indexOf('recently-played-songs')
    const targetIdx = saved.indexOf('most-played-albums')
    expect(sourceIdx).toBeGreaterThan(targetIdx)
  })

  it('skips persisting on dragend when the order has not actually changed', async () => {
    usePreferenceStore().home_blocks_order = blocks.map(block => block.id)
    const saveSpy = vi.spyOn(Storage.prototype, 'setItem')

    const { container } = h.render(Component, { props: { blocks } })
    const rows = Array.from(container.querySelectorAll<HTMLElement>('[draggable="true"]'))

    dispatch(rows[0], 'dragstart', { dataTransfer: { effectAllowed: '' } })
    dispatch(rows[0], 'dragend')
    await nextTick()

    expect(saveSpy).not.toHaveBeenCalled()
  })

  it('hides a block when it is unticked, and shows it again', async () => {
    h.render(Component, { props: { blocks } })

    await h.user.click(screen.getByRole('checkbox', { name: 'Show Top albums' }))
    expect(usePreferenceStore().home_blocks_hidden).toEqual(['most-played-albums'])
    expect(screen.getByRole<HTMLInputElement>('checkbox', { name: 'Show Top albums' }).checked).toBe(false)

    await h.user.click(screen.getByRole('checkbox', { name: 'Show Top albums' }))
    expect(usePreferenceStore().home_blocks_hidden).toEqual([])
  })

  it('moves a block up and down without dragging', async () => {
    const { container } = h.render(Component, { props: { blocks } })

    await h.user.click(screen.getByRole('button', { name: 'Move Top albums up' }))

    expect(rowIds(container)).toEqual(['Recently played', 'Top albums', 'Latest albums', 'Random songs'])
    expect(usePreferenceStore().home_blocks_order).toEqual([
      'recently-played-songs',
      'most-played-albums',
      'recently-added-albums',
      'random-songs',
    ])

    await h.user.click(screen.getByRole('button', { name: 'Move Recently played down' }))
    expect(rowIds(container)[0]).toBe('Top albums')

    // The first can't go up, the last can't go down.
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Move Top albums up' }).disabled).toBe(true)
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Move Random songs down' }).disabled).toBe(true)
  })

  it('emits close when the Close button is clicked', async () => {
    const { emitted } = h.render(Component, { props: { blocks } })

    await fireEvent.click(screen.getByText('Close'))

    expect(emitted().close).toHaveLength(1)
  })

  it('emits close when Escape is pressed', async () => {
    const { emitted } = h.render(Component, { props: { blocks } })

    await fireEvent.keyDown(screen.getByTestId('reorder-blocks-modal'), { key: 'Escape' })

    expect(emitted().close).toHaveLength(1)
  })
})
