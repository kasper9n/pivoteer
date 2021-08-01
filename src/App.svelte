<script lang="ts">
  // import * as tauri from '@tauri-apps/api/tauri'
  import { event } from '@tauri-apps/api'
  import { popup } from './scripts/helpers'
  import { onDestroy } from 'svelte'
  import type { Project } from './scripts/project'
  import { getDefaultProject } from './scripts/project'
  import Main from './components/Project.svelte'

  let currentProject: Project | null = null

  async function newProject() {
    currentProject = getDefaultProject()
  }

  const unlistenFuture = event.listen('menu', ({ payload }) => {
    if (payload === 'New') {
      newProject().catch(popup)
    }
  })
  onDestroy(async () => {
    const unlisten = await unlistenFuture
    unlisten()
  })
</script>

{#if currentProject}
  <Main project={currentProject} />
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
    background-color: #191B20
    color: white
    height: 100%
  :global(h1), :global(h2), :global(h3)
    margin: 0px
    font-weight: normal
  .start-page
    display: flex
    height: 100%
    align-items: center
    justify-content: center
</style>
