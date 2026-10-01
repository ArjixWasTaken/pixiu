import { describe, expect, it } from 'vite-plus/test'
import { isStreamUrl, streamCacheKey, streamSongId } from './streamCache'

describe('streamCache', () => {
  const stream = (params: string) => `https://music.example/rest/stream?${params}`

  it('keeps a stream by its song, format and bitrate, not by who asked', () => {
    const mine = stream('apiKey=one&v=1.16.1&c=pixiu-web&f=json&id=tr-7&format=mp3&maxBitRate=128')
    const theirs = stream('id=tr-7&maxBitRate=128&format=mp3&apiKey=two&c=other&v=1.15.0')

    expect(streamCacheKey(mine)).toBe(stream('format=mp3&id=tr-7&maxBitRate=128'))
    expect(streamCacheKey(theirs)).toBe(streamCacheKey(mine))
    expect(streamCacheKey(stream('apiKey=one&id=tr-7'))).not.toBe(streamCacheKey(mine))
  })

  it('tells a stream and its song', () => {
    expect(isStreamUrl(new URL(stream('id=tr-7')))).toBe(true)
    expect(isStreamUrl(new URL('https://music.example/rest/stream.view?id=tr-7'))).toBe(true)
    expect(isStreamUrl(new URL('https://music.example/rest/getCoverArt?id=al-1'))).toBe(false)
    expect(streamSongId(stream('apiKey=one&id=tr-7'))).toBe('tr-7')
  })
})
