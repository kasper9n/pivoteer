<script lang="ts">
  import type { Project, Source } from '../bindings'
  import Modal from './Modal.svelte'
  import Options from './sources/Source.svelte'
  import Settings from './settings/Settings.svelte'

  export let project: Project
  export let sourceIndex: number | null = null

  let newSourceName: string | null = null
  function addSource() {
    if (newSourceName) {
      const newSource: Source = {
        name: newSourceName,
        columns: [],
        files: [],
        headerRowIndex: 0,
      }
      project.sources = [...project.sources, newSource]
      sourceIndex = project.sources.length - 1
    }
    newSourceName = null
  }
</script>

<div class="container">
  <aside>
    <div class="header" class:active={sourceIndex === null} on:click={() => (sourceIndex = null)}>
      Settings
    </div>
    <div class="header">
      Sources
      <button on:click={() => (newSourceName = '')}>New Source</button>
    </div>
    <div class="fullbox">
      {#each project.sources as source, i}
        <div class="row" class:active={i === sourceIndex} on:click={() => (sourceIndex = i)}>
          {source.name}
        </div>
      {/each}
    </div>
  </aside>
  <main>
    {#if sourceIndex === null}
      <Settings bind:project />
    {:else if project.sources[sourceIndex]}
      <Options bind:source={project.sources[sourceIndex]} />
    {/if}
  </main>
</div>

<Modal
  showIf={newSourceName !== null}
  onClose={() => {
    newSourceName = null
  }}
>
  <form on:submit|preventDefault={addSource}>
    <h3>New Source</h3>
    Name
    <input type="text" bind:value={newSourceName} />
    <div>
      <button type="button" on:click={() => (newSourceName = null)}>Cancel</button>
      <button type="submit" on:click={addSource}>Add</button>
    </div>
  </form>
</Modal>

<style lang="sass">
  .container
    height: 100%
    display: flex
  aside
    width: 30%
    max-width: 300px
    display: flex
    flex-direction: column
    height: 100%
    padding: 0px 10px
    float: left
  .header
    padding: 10px
  .fullbox
    overflow-y: auto
    height: 100%
    margin-bottom: 10px
    border: 1px solid #D1D7DD
    .row
      padding: 4px 10px
      font-size: 14px
      cursor: default
  .active
    background-color: hsla(0, 0%, 100%, 0.1)
  main
    display: flex
    flex-direction: column
    width: 10px
    flex-grow: 1
</style>
