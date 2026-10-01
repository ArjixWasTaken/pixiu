/**
 * The Material 3 color schemes (css/m3/fig-tokens.css). Each has a light
 * and a dark variant; the swatch shows the dark scheme's primary color.
 * "From what's playing" builds one from the cover playing (useCoverTheme),
 * over Orange when there's none.
 */
const themes: Theme[] = [
  {
    id: 'cover',
    name: 'From what’s playing',
    thumbnail_color: 'conic-gradient(rgb(255,182,140), rgb(208,188,255), rgb(254,176,209), rgb(255,182,140))',
  },
  { id: 'orange', name: 'Orange', thumbnail_color: 'rgb(255,182,140)' },
  { id: 'baseline', name: 'Baseline', thumbnail_color: 'rgb(208,188,255)' },
  { id: 'red', name: 'Red', thumbnail_color: 'rgb(255,167,155)' },
  { id: 'rose', name: 'Rose', thumbnail_color: 'rgb(255,178,189)' },
  { id: 'pink', name: 'Pink', thumbnail_color: 'rgb(254,176,209)' },
  { id: 'monochrome', name: 'Monochrome', thumbnail_color: 'rgb(255,255,255)' },
]

export default themes
