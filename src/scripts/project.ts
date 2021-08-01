export type Project = {
  files: string[]
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
    columns: [
      { action: 'Unique', idType: 'Name', id: 'ISRC' },
      { action: 'Unique', idType: 'Name', id: 'UPC' },
      { action: 'Sum', idType: 'Name', id: 'Net earnings (USD)' },
    ],
  }
}
