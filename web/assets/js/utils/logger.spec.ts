import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { logger } from './logger'

describe('logger', () => {
  createHarness()

  it('logs with [píxiū] prefix', () => {
    const spy = vi.spyOn(console, 'log').mockImplementation(() => {})
    logger.log('hello')
    expect(spy).toHaveBeenCalledWith('[píxiū]', 'hello')
    spy.mockRestore()
  })

  it('logs errors with [píxiū] prefix', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {})
    logger.error('something broke')
    expect(spy).toHaveBeenCalledWith('[píxiū]', 'something broke')
    spy.mockRestore()
  })

  it('logs info with [píxiū] prefix', () => {
    const spy = vi.spyOn(console, 'info').mockImplementation(() => {})
    logger.info('note')
    expect(spy).toHaveBeenCalledWith('[píxiū]', 'note')
    spy.mockRestore()
  })

  it('passes extra arguments through', () => {
    const spy = vi.spyOn(console, 'log').mockImplementation(() => {})
    logger.log('msg', { detail: 1 }, 'extra')
    expect(spy).toHaveBeenCalledWith('[píxiū]', 'msg', { detail: 1 }, 'extra')
    spy.mockRestore()
  })
})
