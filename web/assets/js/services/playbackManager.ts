import type { QueuePlaybackService } from '@/services/QueuePlaybackService'
import { playbackService as queuePlayback } from '@/services/QueuePlaybackService'

export const playbackManager = {
  currentService: null as QueuePlaybackService | null,

  useQueuePlayback(mediaElement?: HTMLMediaElement) {
    this.currentService = queuePlayback

    return queuePlayback.activate(mediaElement ?? document.querySelector<HTMLMediaElement>('#audio-player')!)
  },
}

interface PlaybackTypeMap {
  queue: QueuePlaybackService
  current: QueuePlaybackService | null
}

export function playback<T extends keyof PlaybackTypeMap = 'queue'>(
  type?: T,
  mediaElement?: HTMLMediaElement,
): PlaybackTypeMap[T] {
  if (type === 'current') {
    return playbackManager.currentService as PlaybackTypeMap[T]
  }

  return playbackManager.useQueuePlayback(mediaElement) as PlaybackTypeMap[T]
}
