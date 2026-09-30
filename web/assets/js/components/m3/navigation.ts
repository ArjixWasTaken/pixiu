/** An entry of a navigation drawer, rail or bar. */
export interface M3NavItem {
  id: string
  label: string
  icon: string
  /** The icon when the item is selected; the filled `icon` by default. */
  activeIcon?: string
  badge?: string | number
  /** An icon shown at the end, like a folder's expand arrow. */
  trailingIcon?: string
  spin?: boolean
  href?: string
}

export interface M3NavHeader {
  header: string
}

export type M3NavEntry = M3NavItem | M3NavHeader

export const isHeader = (entry: M3NavEntry): entry is M3NavHeader => 'header' in entry
