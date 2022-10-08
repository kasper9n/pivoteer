<script lang="ts">
  import type { ColumnConfig } from '../../bindings'

  export let label: string
  export let column: ColumnConfig | null

  let columnTypes: { text: string; value: ColumnConfig | null }[] = [
    { text: 'Disabled', value: null },
    { text: 'Named', value: { Name: '' } },
    { text: 'At index', value: { Index: 0 } },
    { text: 'Named at index', value: { NameAtIndex: ['', 0] } },
    { text: 'Fill all with value', value: { CustomValue: '' } },
  ]
  function getIndex(c: ColumnConfig | null) {
    if (c === null) return 0
    else if ('Name' in c) return 1
    else if ('Index' in c) return 2
    else if ('NameAtIndex' in c) return 3
    else if ('CustomValue' in c) return 4
    else return 0
  }
  let index = getIndex(column)
  $: if (getIndex(column) !== index) {
    column = columnTypes[index].value
  }
</script>

<div class="row">
  <div>{label}</div>
  <select bind:value={index}>
    {#each columnTypes as columnType, i}
      <option value={i}>{columnType.text}</option>
    {/each}
  </select>
  {#if column === null}
    <!-- Noop -->
  {:else if 'Name' in column}
    <input type="text" bind:value={column.Name} placeholder={label} />
  {:else if 'Index' in column}
    <input type="number" bind:value={column.Index} placeholder="0" />
  {:else if 'NameAtIndex' in column}
    <input type="text" bind:value={column.NameAtIndex[0]} placeholder={label} />
    <input type="number" bind:value={column.NameAtIndex[1]} placeholder="0" />
  {:else if 'CustomValue' in column}
    <input type="text" bind:value={column.CustomValue} placeholder="Value" />
  {/if}
</div>

<style lang="sass">
  .row
    display: flex
    align-items: center
    font-size: 14px
  input[type='number']
    width: 50px
  select
    font-size: inherit
</style>
