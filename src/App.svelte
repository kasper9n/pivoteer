<script lang="ts">
  import { dialog, event, window as tauriWindow } from '@tauri-apps/api'
  import { onDestroy } from 'svelte'
  import type { Project } from './bindings'
  import ProjectComponent from './components/Project.svelte'
  import { runCmd } from './scripts/helpers'

  type File = {
    path?: string
    project: Project
  }
  let file: File | null = null
  $: file?.project, setEdited(!!file?.project)

  runCmd('set_edited', { edited: false })
  let isEdited = false
  async function setEdited(edited: boolean) {
    if (isEdited !== edited) {
      await runCmd('set_edited', { edited })
      isEdited = edited
    }
  }

  async function newProject() {
    file = {
      project: {
        columns: [
          { name: 'ISRC', kind: 'Isrc', action: 'Unique', enabled: true },
          { name: 'UPC', kind: 'Upc', action: 'Unique', enabled: true },
          { name: 'Revenue', kind: 'NetEarnings', action: 'Sum', enabled: true },
        ],
        sources: [
          {
            name: 'Landr',
            kind: { id: 'Landr' },
            files: [],
          },
        ],
      },
    }
  }

  async function openProjectDialog() {
    const filePath = await dialog.open({
      filters: [{ name: 'Pivoteer', extensions: ['pivoteer'] }],
      multiple: false,
    })
    if (typeof filePath === 'string') {
      openProject(filePath)
    }
  }
  async function openProject(path: string) {
    file = {
      path,
      project: await runCmd('open', { path }),
    }
  }

  async function saveProject(file: File) {
    if (!file.path) {
      const pickedPath = await dialog.save({
        filters: [{ name: 'Pivoteer', extensions: ['pivoteer'] }],
      })
      file.path = pickedPath || undefined
    }
    if (file.path) {
      await runCmd('save', { project: file.project, path: file.path })
      setEdited(false)
    }
  }

  const unlistenFuture = event.listen('menu', ({ payload }) => {
    if (payload === 'New') {
      newProject()
    } else if (payload === 'Open...') {
      openProjectDialog()
    } else if (payload === 'Close') {
      if (file) {
        file = null
      } else {
        tauriWindow.getCurrent().close()
      }
    } else if (payload === 'Save' && file) {
      saveProject(file)
    }
  })
  onDestroy(async () => {
    const unlisten = await unlistenFuture
    unlisten()
  })
</script>

{#if file}
  <ProjectComponent bind:project={file.project} />
{:else}
  <div class="start-page">
    <div class="col">
      <button on:click={newProject}>New Project</button>
      <button on:click={openProjectDialog}>Open...</button>
    </div>
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
  .col
    display: flex
    flex-direction: column
    align-items: start
    // justify-content: start
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
