<script lang="ts">
  import type { InputColumn } from '../scripts/project'

  export let column: InputColumn
  export let remove: () => void
  function uintFilterNonZero(value: string) {
    return value.replace(/0*[^0-9]*/g, '')
  }
  $: if (column.idType === 'Number') {
    column.id = uintFilterNonZero(column.id)
  }
</script>

<div class="col">
  <select bind:value={column.idType}>
    <option value="Name">Name:</option>
    <option value="Number">Number:</option>
  </select>
  {#if column.idType === 'Name'}
    <input type="text" bind:value={column.id} placeholder="Column header" />
  {:else}
    <input type="text" bind:value={column.id} placeholder="1" />
  {/if}
  Action:
  <select bind:value={column.action}>
    <option value="Unique">Keep unique values</option>
    <option value="Sum">Sum</option>
  </select>
  <button on:click={remove}>-</button>
</div>

<style lang="sass">
  .col
    font-size: 12px
  input
    font-size: inherit
  select
    font-size: inherit
</style>
