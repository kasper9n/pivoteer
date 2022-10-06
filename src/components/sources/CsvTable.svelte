<script>
  export let csv
  export let csvRowDelimiter = '\n'
  export let csvColumnDelimiter = ','
  export let hasHeader = true
  export let tableClass = ''
  export let tableRowClass = ''
  export let tableColumnClass = ''
  $: rows = (() => {
    let rows = csv ? csv.split(csvRowDelimiter) : null
    if (rows[rows.length - 1] === '') rows.pop()
    return rows
  })()
  $: table = rows ? rows.map((row) => row.split(csvColumnDelimiter)) : []
  $: header = hasHeader && table && table.length ? table[0] : null
  $: body = table && table.length ? (hasHeader ? table.slice(1, table.length) : table) : null
</script>

<table class={tableClass}>
  {#if header}
    <thead>
      <tr>
        {#each header as column, i}
          <th key={`table-col-${i}`}>{column}</th>
        {/each}
      </tr>
    </thead>
  {/if}
  {#if body}
    <tbody>
      {#each body as row, rowI}
        <tr class={tableRowClass} key={`${row ? row : 'row'}-${rowI}`}>
          {#each row as column, colI}
            <td class={tableColumnClass} key={`${column}-${colI}-'${rowI}`}>
              {column}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  {/if}
</table>

<style lang="sass">
  table
    max-height: 100%
    font-size: 14px
    border-collapse: collapse
    background-color: #FFFFFF
  th, td
    border: 1px solid #D1D7DD
    padding: 6px 13px
  tr:nth-child(2n)
    background-color: #F6F8FA
</style>
