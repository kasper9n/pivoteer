<script lang="ts">
  import type { Source } from '../scripts/project'
  import ColumnOption from './ColumnOption.svelte'

  export let source: Source
  let headerRowNumber: number | null = 1
  $: {
    if (headerRowNumber === null) {
      source.headerRowIndex = 1
    } else {
      if (headerRowNumber < 1) {
        headerRowNumber = 1
      } else if (headerRowNumber % 1 !== 0) {
        headerRowNumber = Math.floor(headerRowNumber)
      }
      source.headerRowIndex = headerRowNumber - 1
    }
  }
  function addColumn() {
    source.columns.push({
      idType: 'Name',
      id: '',
    })
    source.columns = source.columns
  }
  function removeColumn(index: number) {
    source.columns.splice(index, 1)
    source.columns = source.columns
  }
</script>

<div class="container">
  <h3>Options</h3>
  <div class="box">
    <div class="row">
      <p>Header row number:</p>
      <input type="number" bind:value={headerRowNumber} placeholder="1" />
    </div>
    <div class="row">
      <p>Columns</p>
      <button on:click={addColumn}>+</button>
    </div>
    {#each source.columns as column, i}
      <ColumnOption {column} remove={() => removeColumn(i)} />
    {/each}
  </div>
  <h3>Files</h3>
  <div class="box">
    {#each source.files as file}
      <p>{file.replace(/^.*[\\/]/, '')}</p>
    {/each}
  </div>
</div>

<style lang="sass">
  p
    margin: 0px
  .row
    display: flex
    align-items: center
  input
    margin-left: 5px
    width: 35px
  .box
    border: 1px solid #D1D7DD
    padding: 10px
    display: block
</style>
