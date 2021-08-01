<script lang="ts">
  import type { Project } from '../scripts/project'
  import ColumnOption from './ColumnOption.svelte'

  export let project: Project
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
  <div class="header">
    <h3>Columns</h3>
    <button on:click={addColumn}>+</button>
  </div>
  {#each project.columns as column, i}
    <ColumnOption {column} remove={() => removeColumn(i)} />
  {/each}
</div>

<style lang="sass">
  .container
    width: 100%
  .header
    display: flex
</style>
