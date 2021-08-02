<script lang="ts">
  // import * as tauri from '@tauri-apps/api/tauri'
  import { event } from '@tauri-apps/api'
  import { popup } from './scripts/helpers'
  import { onDestroy } from 'svelte'
  import type { Project } from './scripts/project'
  import { getDefaultProject } from './scripts/project'
  import Main from './components/Project.svelte'
  import FileDrop from './components/FileDrop.svelte'

  let project: Project | null = null

  async function newProject() {
    project = getDefaultProject()
  }
  async function addFiles(files: string[]) {
    if (project === null) {
      project = getDefaultProject()
    }
    for (const file of files) {
      if (project.files.includes(file)) {
        await popup('Skipping duplicate file: ' + file)
      } else {
        project.files.push(file)
        project.files = project.files
      }
    }
  }
  $: msg = project === null ? 'Drop files to add in new project' : 'Drop files to add'

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

<FileDrop fileExtensions={['csv', 'tsv']} handleFiles={addFiles} {msg} />

<style lang="sass">
  :global(html)
    height: 100%
    box-sizing: border-box
    overflow: hidden
  :global(body)
    margin: 0
    font-family: Arial, Helvetica, sans-serif
    font-size: 18px
    background-color: #f8f9fc
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
</style>
