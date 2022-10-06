<script lang="ts">
  import type { Project } from '../../scripts/project'
  import OutputColumn from './OutputColumn.svelte'

  export let project: Project

  function addColumn() {
    project.columns.push({
      name: '',
      action: 'Unique',
    })
    project.columns = project.columns
  }
  function removeColumn(index: number) {
    project.columns.splice(index, 1)
    project.columns = project.columns
  }
</script>

<h3>Settings</h3>
<div class="row">
  <p>Columns</p>
  <button on:click={addColumn}>+</button>
</div>
{#each project.columns as column, i}
  <OutputColumn bind:column remove={() => removeColumn(i)} />
{/each}

<style lang="sass">
  .row
    display: flex
    align-items: center
</style>
