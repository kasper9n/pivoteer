<script lang="ts">
  import type { Column } from 'src/scripts/instance'

  export let column: Column
  export let remove: () => void
  function uintFilter(value: string) {
    return value.replace(/[^0-9]*/g, '')
  }
  $: if (column.idType === 'number') {
    column.id = uintFilter(column.id)
  }
</script>

<div class="col">
  <select bind:value={column.idType}>
    <option value="name">Name:</option>
    <option value="number">Number:</option>
  </select>
  {#if column.idType === 'name'}
    <input type="text" bind:value={column.id} placeholder="Column header" />
  {:else}
    <input type="text" bind:value={column.id} placeholder="1" />
  {/if}
  Action:
  <select bind:value={column.action}>
    <option value="unique">Keep unique values</option>
    <option value="sum">Sum</option>
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
