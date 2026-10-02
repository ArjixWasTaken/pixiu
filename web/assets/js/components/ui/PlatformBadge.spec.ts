import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './PlatformBadge.vue'

describe('platformBadge.vue', () => {
  const h = createHarness()

  it('shows the mark of the platform a song came from', () => {
    const { container } = h.render(Component, { props: { platform: 'youtube_music' } })

    const badge = container.querySelector('img')!
    expect(badge.getAttribute('title')).toBe('From YouTube Music')
    expect(badge.getAttribute('aria-hidden')).toBe('true')
    expect(badge.classList.contains('sm')).toBe(true)
  })

  it('knows Deezer too', () => {
    const { container } = h.render(Component, { props: { platform: 'deezer', size: 'lg' } })

    const badge = container.querySelector('img')!
    expect(badge.getAttribute('title')).toBe('From Deezer')
    expect(badge.getAttribute('src')).toMatch(/deezer/)
  })

  it('shows nothing for uploads and platforms it does not know', () => {
    for (const platform of [null, 'myspace']) {
      const { container, unmount } = h.render(Component, { props: { platform } })
      expect(container.querySelector('img')).toBeNull()
      unmount()
    }
  })
})
