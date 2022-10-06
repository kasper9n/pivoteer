<script lang="ts">
  import type { Column } from '../../scripts/project'

  export let column: Column
  export let remove: () => void
</script>

<div class="col">
  Name:
  <input type="text" bind:value={column.name} placeholder="Column header" />
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
  select
    font-size: inherit
</style>
