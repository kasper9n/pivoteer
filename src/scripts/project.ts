export type Project = {
  files: string[]
  headerRowIndex: number
  columns: Column[]
}
export type Column = {
  action: 'Unique' | 'Sum'
  idType: 'Name' | 'Number'
  id: string
}

export function getDefaultProject(): Project {
  return {
    files: [],
    headerRowIndex: 1,
    columns: [
      { action: 'Unique', idType: 'Name', id: 'ISRC' },
      { action: 'Unique', idType: 'Name', id: 'UPC' },
      { action: 'Sum', idType: 'Name', id: 'Net earnings (USD)' },
    ],
  }
}
