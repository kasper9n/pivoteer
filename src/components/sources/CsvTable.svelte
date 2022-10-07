<script lang="ts">
  export let csv: string
  export let csvRowDelimiter = '\n'
  export let csvColumnDelimiter = ','
  export let hasHeader = true
  $: rows = (() => {
    if (csv) {
      let rows = csv.split(csvRowDelimiter)
      if (rows[rows.length - 1] === '') rows.pop()
      return rows
    } else {
      return null
    }
  })()
  $: table = rows ? rows.map((row) => row.split(csvColumnDelimiter)) : []
  $: header = hasHeader && table && table.length ? table[0] : null
  $: body = table && table.length ? (hasHeader ? table.slice(1, table.length) : table) : null
</script>

<table>
  {#if header}
    <thead>
      <tr>
        {#each header as column, i}
          <th>{column}</th>
        {/each}
      </tr>
    </thead>
  {/if}
  {#if body}
    <tbody>
      {#each body as row}
        <tr>
          {#each row as column}
            <td>
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
  th, td
    border: 1px solid #D1D7DD
    padding: 6px 13px
  tr
    background-color: hsl(220, 60%, 57%, 0.2)
  tbody tr:nth-child(2n+1)
    background-color: hsl(220, 60%, 57%, 0.1)
</style>
