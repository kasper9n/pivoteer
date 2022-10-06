export function getDefaultProject(): Project {
  return {
    sources: [],
    columns: [],
  }
}

export type Project = {
  sources: Source[]
  columns: OutputColumn[]
}

export type Source = {
  name: string
  files: string[]
  headerRowIndex: number
  columns: InputColumn[]
}
export type InputColumn = {
  idType: 'Name' | 'Number'
  id: string
}

export type OutputColumn = {
  name: string
  action: 'Unique' | 'Sum'
}
