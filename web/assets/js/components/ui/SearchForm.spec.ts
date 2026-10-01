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

  it('goes to search screen when search box is focused', async () => {
    const mock = h.mock(Router, 'go')
    h.render(Component)

    await h.user.click(screen.getByRole('searchbox'))

    expect(mock).toHaveBeenCalledWith('/search')
  })

  it('emits an event when search query is changed', async () => {
    const mock = h.mock(eventBus, 'emit')
    h.render(Component)

    await h.type(screen.getByRole('searchbox'), 'hey')

    expect(mock).toHaveBeenCalledWith('SEARCH_KEYWORDS_CHANGED', 'hey')
  })

  it('goes to the search screen if the form is submitted', async () => {
    const goMock = h.mock(Router, 'go')
    h.render(Component)

    await h.type(screen.getByRole('searchbox'), 'hey{Enter}')

    expect(goMock).toHaveBeenCalledWith('/search')
  })

  it('leaves the search behind when leaving the results', async () => {
    h.render(Component)
    await h.visit('/search')
    await h.type(screen.getByRole('searchbox'), 'coldplay')

    await h.visit('/search')
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('coldplay')

    await h.visit('/home')
    await h.tick()
    expect(screen.getByRole<HTMLInputElement>('searchbox').value).toBe('')
  })
})
