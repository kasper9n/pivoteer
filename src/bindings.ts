import type { SourceType } from '../bindings'

export * from '../bindings'

export const sourceTypes: { type: SourceType; name: string }[] = [
  { type: 'Landr', name: 'Landr' },
  { type: 'Pretzel', name: 'Pretzel' },
  { type: 'RepostBySoundCloud', name: 'Repost By SoundCloud' },
]
