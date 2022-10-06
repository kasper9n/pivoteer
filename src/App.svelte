<script lang="ts">
  import { event } from '@tauri-apps/api'
  import { onDestroy } from 'svelte'
  import type { Project } from './scripts/project'
  import { getDefaultProject } from './scripts/project'
  import Main from './components/Project.svelte'

  let project: Project | null = null

  async function newProject() {
    project = getDefaultProject()
  }

  const unlistenFuture = event.listen('menu', ({ payload }) => {
    if (payload === 'New') {
      newProject()
    }
  })
  onDestroy(async () => {
    const unlisten = await unlistenFuture
    unlisten()
  })
</script>

{#if project}
  <Main {project} />
{:else}
  <div class="start-page">
    <button on:click={newProject}>New Project</button>
  </div>
{/if}

<style lang="sass">
  :global(html)
    height: 100%
    box-sizing: border-box
    overflow: hidden
  :global(body)
    margin: 0
    font-family: Arial, Helvetica, sans-serif
    font-size: 18px
    background-color: #100711
    background: linear-gradient(#42151E, #100711)
    color: white
    height: 100%
  :global(h1), :global(h2), :global(h3)
    margin: 0px
    font-weight: normal
  :global(p)
    font-size: 14px
  .start-page
    display: flex
    height: 100%
    align-items: center
    justify-content: center
  button
    font-size: 15px
    padding: 8px 20px
    border: none
    border-radius: 5px
    background-color: hsla(305, 41%, 38%, 0.3)
    color: white
    &:hover
      background-color: hsla(305, 41%, 38%, 0.5)
</style>
