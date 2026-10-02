import youTubeMusicLogo from '@/../img/platforms/youtube-music.svg'
import deezerLogo from '@/../img/platforms/deezer.svg'

export interface PlatformInfo {
  /** The platform's name, for people. */
  name: string
  /** Its mark, for badges on covers. */
  logo: string
}

/** The platforms songs are downloaded from, by the id the server names them by. */
export const platforms: Record<string, PlatformInfo> = {
  youtube_music: { name: 'YouTube Music', logo: youTubeMusicLogo },
  deezer: { name: 'Deezer', logo: deezerLogo },
}

/** A platform's name, for people; the id itself when the player doesn't know it. */
export const platformName = (id: string | null | undefined) => (id && platforms[id]?.name) || id || ''
