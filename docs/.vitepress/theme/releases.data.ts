import { createContentLoader } from 'vitepress'

export type ReleaseChanges = {
  nieuw?: string[]
  verbeterd?: string[]
  opgelost?: string[]
}

export type ReleaseImage = {
  src: string
  tab?: string
  alt: string
  caption: string
}

export type ReleaseHighlight = {
  icon: string
  title: string
  text: string
}

export type ReleaseText = {
  title: string
  label: string | null
  summary: string
  images: ReleaseImage[]
  highlights: ReleaseHighlight[]
  changes: ReleaseChanges
}

export type Release = ReleaseText & {
  version: string
  date: string
  url: string
  building: boolean
  en: Partial<ReleaseText>
}

declare const data: Release[]
export { data }

export function versionKey(version: string) {
  return version
    .replace(/^v/, '')
    .split('.')
    .map((part) => part.padStart(5, '0'))
    .join('.')
}

export default createContentLoader('releases/v*.md', {
  transform(pages): Release[] {
    return pages
      .map((page) => ({
        version: page.frontmatter.version as string,
        title: page.frontmatter.title as string,
        label: (page.frontmatter.label as string | undefined) ?? null,
        summary: page.frontmatter.summary as string,
        date: page.frontmatter.date as string,
        url: page.url,
        images: (page.frontmatter.images ?? []) as ReleaseImage[],
        highlights: (page.frontmatter.highlights ?? []) as ReleaseHighlight[],
        changes: (page.frontmatter.changes ?? {}) as ReleaseChanges,
        building: page.frontmatter.status === 'building',
        en: (page.frontmatter.en ?? {}) as Partial<ReleaseText>,
      }))
      .sort((left, right) => versionKey(right.version).localeCompare(versionKey(left.version)))
  },
})
