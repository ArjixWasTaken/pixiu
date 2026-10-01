import { screen } from '@testing-library/vue'
import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './ArtistAlbumScreenTabs.vue'

describe('ArtistAlbumScreenTabs', () => {
  const h = createHarness()

  const tabs = [
    { id: 'songs', label: 'Songs' },
    { id: 'albums', label: 'Albums' },
  ]

  it('renders the tabs and the panels', () => {
    h.render(Component, {
      props: { tabs, idPrefix: 'artist', modelValue: 'songs' },
      slots: { default: '<div>Tab content</div>' },
    })

    screen.getByRole('tab', { name: 'Songs', selected: true })
    screen.getByRole('tab', { name: 'Albums', selected: false })
    screen.getByText('Tab content')
  })

  it('selects a tab with the arrow keys', async () => {
    const { emitted } = h.render(Component, { props: { tabs, idPrefix: 'artist', modelValue: 'songs' } })

    screen.getByRole('tab', { name: 'Songs' }).focus()
    await h.user.keyboard('{ArrowRight}')

    expect(document.activeElement).toBe(screen.getByRole('tab', { name: 'Albums' }))
    expect(emitted('update:modelValue')).toEqual([['albums']])
  })
})
