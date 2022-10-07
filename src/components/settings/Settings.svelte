<script lang="ts">
  import type { Project } from '../../bindings'
  import Generate from './Generate.svelte'
  import OutputColumn from './OutputColumn.svelte'

  export let project: Project

  function addColumn() {
    project.columns.push({
      name: '',
      action: 'Unique',
      enabled: true,
      kind: 'Isrc',
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

<h3>Generate</h3>
<Generate {project} />

<style lang="sass">
  .row
    display: flex
    align-items: center
</style>
