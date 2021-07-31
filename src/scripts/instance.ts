export type Instance = {
  files: string[]
  columns: Column[]
}
export type Column = {
  action: 'unique' | 'sum'
  idType: 'name' | 'number'
  id: string
}

export function getDefaultInstance(): Instance {
  return {
    files: [],
    columns: [],
  }
}
