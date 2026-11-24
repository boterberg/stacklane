import { createContentLoader } from 'vitepress'

export type Release = {
  version: string
  title: string
  summary: string
  date: string
  url: string
}

declare const data: Release[]
export { data }

function versionKey(version: string) {
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
        summary: page.frontmatter.summary as string,
        date: page.frontmatter.date as string,
        url: page.url,
      }))
      .sort((left, right) => versionKey(right.version).localeCompare(versionKey(left.version)))
  },
})
