<script lang="ts">
  import type { Instance } from 'src/scripts/instance'
  import ColumnOption from './ColumnOption.svelte'

  export let instance: Instance
  function addColumn() {
    instance.columns.push({
      action: 'unique',
      idType: 'name',
      id: '',
    })
    instance.columns = instance.columns
  }
  function removeColumn(index: number) {
    instance.columns.splice(index, 1)
    instance.columns = instance.columns
  }
</script>

<main>
  <div class="header">
    Columns
    <button on:click={addColumn}>+</button>
  </div>
  {#each instance.columns as column, i}
    <ColumnOption {column} remove={() => removeColumn(i)} />
  {/each}
</main>

<style lang="sass">
  main
    width: 100%
    padding: 10px
</style>
