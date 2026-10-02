import youTubeMusicLogo from '@/../img/platforms/youtube-music.svg'

export interface PlatformInfo {
  /** The platform's name, for people. */
  name: string
  /** Its mark, for badges on covers. */
  logo: string
}

/** The platforms songs are downloaded from, by the id the server names them by. */
export const platforms: Record<string, PlatformInfo> = {
  youtube_music: { name: 'YouTube Music', logo: youTubeMusicLogo },
}
