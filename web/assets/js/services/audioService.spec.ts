import { describe, expect, it, vi } from 'vite-plus/test'
import { createHarness } from '@/__tests__/TestHarness'
import { audioService, dbToGain } from './audioService'

describe('audioService', () => {
  createHarness()

  describe('dbToGain', () => {
    it('converts 0 dB to gain of 1', () => {
      expect(dbToGain(0)).toBe(1)
    })

    it('converts positive dB to gain > 1', () => {
      expect(dbToGain(20)).toBeCloseTo(10)
    })

    it('converts negative dB to gain < 1', () => {
      expect(dbToGain(-20)).toBeCloseTo(0.1)
    })

    it('handles -Infinity as 0 gain', () => {
      expect(dbToGain(-Infinity)).toBe(0)
    })
  })

  describe('setBypassed', () => {
    const wire = () => {
      const source = { connect: vi.fn(), disconnect: vi.fn() }
      Object.assign(audioService, {
        source,
        bypassed: false,
        preampGainNode: { name: 'preamp' },
        analyzer: { name: 'analyzer' },
      })
      return source
    }

    it('plays straight into the analyzer while off, and through the preamp while on', () => {
      const source = wire()

      audioService.setBypassed(true)
      expect(source.disconnect).toHaveBeenCalledOnce()
      expect(source.connect).toHaveBeenLastCalledWith(audioService.analyzer)

      audioService.setBypassed(false)
      expect(source.connect).toHaveBeenLastCalledWith(audioService.preampGainNode)
    })

    it('leaves the wiring alone when nothing changes', () => {
      const source = wire()

      audioService.setBypassed(false)

      expect(source.disconnect).not.toHaveBeenCalled()
      expect(source.connect).not.toHaveBeenCalled()
    })
  })
})
