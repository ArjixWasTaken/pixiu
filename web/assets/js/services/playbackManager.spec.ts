import { describe, expect, it } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { playbackService as queuePlayback } from '@/services/QueuePlaybackService'
import { playback, playbackManager } from '@/services/playbackManager'

describe('playbackManager', () => {
  const h = createHarness({
    beforeEach: () => h.createAudioPlayer(),
  })

  it('uses the queue playback service', () => {
    expect(playbackManager.useQueuePlayback()).toBe(queuePlayback)
    expect(playback('current')).toBe(queuePlayback)
  })

  it('provides shortcuts to it', () => {
    expect(playback()).toBe(queuePlayback)
    expect(playback('queue')).toBe(queuePlayback)
    expect(playback('current')).toBe(queuePlayback)
  })
})
