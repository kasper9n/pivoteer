<script lang="ts">
  import { event } from '@tauri-apps/api'
  import { onDestroy } from 'svelte'
  import type { Project } from './bindings'
  import ProjectComponent from './components/Project.svelte'

  let project: Project | null = null

  async function newProject() {
    project = {
      columns: [
        { name: 'ISRC', kind: 'Isrc', action: 'Unique', enabled: true },
        { name: 'UPC', kind: 'Upc', action: 'Unique', enabled: true },
        { name: 'Revenue', kind: 'NetEarnings', action: 'Unique', enabled: true },
      ],
      sources: [
        {
          name: 'Landr',
          source_type: 'Landr',
          files: [],
        },
      ],
    }
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
  <ProjectComponent {project} />
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
    background: #18181b
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
    background-color: #132C56
    color: #2e7bff
    font-weight: 500
    &:hover
      background-color: #142f5c
</style>
