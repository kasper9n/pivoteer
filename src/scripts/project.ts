export function getDefaultProject(): Project {
  return {
    sources: [
      {
        name: 'Landr',
        headerRowIndex: 0,
        columns: [
          { name: 'ISRC', number: null },
          { name: 'UPC', number: null },
          { name: 'Net earnings (USD)', number: null },
        ],
        files: [],
      },
    ],
    columns: [
      { name: 'ISRC', action: 'Unique' },
      { name: 'UPC', action: 'Unique' },
      { name: 'Revenue', action: 'Sum' },
    ],
  }
}

export type Project = {
  columns: Column[]
  sources: Source[]
}
export type Column = {
  name: string
  action: 'Unique' | 'Sum'
}

export type Source = {
  name: string
  headerRowIndex: number
  /** Indexed by project.column index */
  columns: SourceColumn[]
  files: string[]
}
export type SourceColumn = {
  name: string | null
  number: number | null
}
