import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { eventBus } from './eventBus'

describe('eventBus', () => {
  createHarness({
    beforeEach: () => {
      eventBus.all.clear()
    },
  })

  it('emits and receives events', () => {
    const callback = vi.fn()
    eventBus.on('LOG_OUT', callback)
    eventBus.emit('LOG_OUT')
    expect(callback).toHaveBeenCalledOnce()
  })

  it('passes payload to listeners', () => {
    const callback = vi.fn()
    const playlist = { id: 'pl-1' } as Playlist
    eventBus.on('PLAYLIST_UPDATED', callback)
    eventBus.emit('PLAYLIST_UPDATED', playlist)
    expect(callback).toHaveBeenCalledWith(playlist)
  })

  it('supports multiple listeners', () => {
    const cb1 = vi.fn()
    const cb2 = vi.fn()
    eventBus.on('LOG_OUT', cb1)
    eventBus.on('LOG_OUT', cb2)
    eventBus.emit('LOG_OUT')
    expect(cb1).toHaveBeenCalledOnce()
    expect(cb2).toHaveBeenCalledOnce()
  })

  it('removes listeners', () => {
    const callback = vi.fn()
    eventBus.on('LOG_OUT', callback)
    eventBus.off('LOG_OUT', callback)
    eventBus.emit('LOG_OUT')
    expect(callback).not.toHaveBeenCalled()
  })
})
