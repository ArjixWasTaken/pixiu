import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from '@/utils/eventBus'
import Router from '@/router'
import { setViewport } from '@/composables/useViewport'
import Component from './SearchForm.vue'

describe('searchForm.vue', () => {
  const h = createHarness({
    beforeEach: () => setViewport({ mobile: false }),
    afterEach: () => setViewport({ mobile: true, wide: true }),
  })

  it('sets focus into search box when requested', async () => {
    h.render(Component)

    eventBus.emit('FOCUS_SEARCH_FIELD')

    expect(screen.getByRole('searchbox')).toBe(document.activeElement)
  })

  it('stays put when the search box is only focused', async () => {
    const mock = h.mock(Router, 'go')
    await h.visit('/albums')
    h.render(Component)

    await h.user.click(screen.getByRole('searchbox'))

    expect(mock).not.toHaveBeenCalled()
  })

  it('opens the results for what is typed', async () => {
    const mock = h.mock(Router, 'go')
    await h.visit('/home')
    h.render(Component)

    await h.type(screen.getByRole('searchbox'), 'hey')

    expect(mock).toHaveBeenCalledWith('/search?q=hey')
  })

  it('opens the results for the words when the form is submitted', async () => {
    const goMock = h.mock(Router, 'go')
    h.render(Component)

    await h.type(screen.getByRole('searchbox'), 'lo & behold{Enter}')

    expect(goMock).toHaveBeenLastCalledWith('/search?q=lo%20%26%20behold')
  })

  it('shows the words of the results it is on, and leaves them behind elsewhere', async () => {
    h.render(Component)
    await h.visit('/search?q=coldplay')
    await h.tick()
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('coldplay')

    await h.visit('/home')
    await h.tick()
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('')

    // Back to the results, as with the Back button.
    await h.visit('/search?q=coldplay')
    await h.tick()
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('coldplay')
  })
})
