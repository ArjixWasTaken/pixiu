import { describe, expect, it } from 'vite-plus/test'
import { screen } from '@testing-library/vue'
import { createHarness } from '@/__tests__/TestHarness'
import Component from './PlayableThumbnail.vue'

describe('playableThumbnail.vue', () => {
  const h = createHarness()

  const renderComponent = (playbackState: PlaybackState = 'Stopped') => {
    const playable = h.factory('song').make({
      playback_state: playbackState,
      play_count: 10,
      title: 'Foo bar',
    })

    const rendered = h.render(Component, {
      props: {
        playable,
      },
    })

    return {
      ...rendered,
      playable,
    }
  }

  it('marks the platform a song was downloaded from, not uploads', () => {
    const downloaded = h.factory('song').make({ source_platform: 'youtube_music' })
    const { container, unmount } = h.render(Component, { props: { playable: downloaded } })
    expect(container.querySelector('img[title="From YouTube Music"]')).not.toBeNull()
    unmount()

    const uploaded = h.factory('song').make({ source_platform: null })
    const rendered = h.render(Component, { props: { playable: uploaded } })
    expect(rendered.container.querySelector('img[title]')).toBeNull()
  })

  it('shows no mark in place of the cover, when numbered', () => {
    const playable = h.factory('song').make({ source_platform: 'youtube_music' })
    const { container } = h.render(Component, { props: { playable, numbered: true } })
    expect(container.querySelector('img[title="From YouTube Music"]')).toBeNull()
  })

  it('emits the event when clicked', async () => {
    h.createAudioPlayer()
    const { emitted } = renderComponent()
    await h.user.click(screen.getByRole('button'))

    expect(emitted().clicked).toBeTruthy()
  })
})
