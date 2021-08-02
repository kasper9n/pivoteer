<script lang="ts">
  import type { Project } from '../scripts/project'
  import ColumnOption from './ColumnOption.svelte'

  export let project: Project
  let headerRowNumber: number | null = 1
  $: {
    if (headerRowNumber === null) {
      project.headerRowIndex = 1
    } else {
      if (headerRowNumber < 1) {
        headerRowNumber = 1
      } else if (headerRowNumber % 1 !== 0) {
        headerRowNumber = Math.floor(headerRowNumber)
      }
      project.headerRowIndex = headerRowNumber - 1
    }
  }
  function addColumn() {
    project.columns.push({
      action: 'Unique',
      idType: 'Name',
      id: '',
    })
    project.columns = project.columns
  }
  function removeColumn(index: number) {
    project.columns.splice(index, 1)
    project.columns = project.columns
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
    {#each project.columns as column, i}
      <ColumnOption {column} remove={() => removeColumn(i)} />
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
    // display: flex
    display: block
</style>
