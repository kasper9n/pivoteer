<script lang="ts">
  import { dialog, invoke, fs } from '@tauri-apps/api'
  import * as clipboard from '@tauri-apps/api/clipboard'
  import { popup } from '../scripts/helpers'
  import type { Project, Source } from '../scripts/project'
  import CsvTable from './CsvTable.svelte'
  import FileDrop from 'svelte-tauri-filedrop'
  import Modal from './Modal.svelte'
  import Options from './Options.svelte'
  import { fade } from 'svelte/transition'

  export let project: Project
  export let sourceIndex = 0

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

  async function addFiles(files: string[]) {
    if (!project.sources[sourceIndex]) {
      return
    }
    const source = project.sources[sourceIndex]
    for (const file of files) {
      if (source.files.includes(file)) {
        await popup('Skipping duplicate file: ' + file)
      } else {
        source.files.push(file)
        project.sources[sourceIndex].files = project.sources[sourceIndex].files
      }
    }
  }
  async function addFilesDialog() {
    const filePaths = await dialog.open({
      filters: [
        { name: 'CSV', extensions: ['csv'] },
        { name: 'TSV', extensions: ['tsv'] },
      ],
      multiple: true,
    })
    if (filePaths instanceof Array) {
      addFiles(filePaths)
    }
  }
  let outputCsv: string | null = null
  let generating = false
  async function generate() {
    if (generating || !project.sources[sourceIndex]) {
      return
    }
    try {
      generating = true
      outputCsv = await invoke('generate', { source: project.sources[sourceIndex] })
    } catch (err) {
      popup(String(err))
      outputCsv = null
    }
    generating = false
  }
  function csvCopy() {
    if (outputCsv === null) return
    clipboard.writeText(outputCsv)
  }
  async function csvSaveAs() {
    try {
      if (outputCsv === null) return
      const filePath = await dialog.save({
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      })
      if (!filePath) return
      await fs.writeFile({
        path: filePath,
        contents: outputCsv,
      })
    } catch (err) {
      popup(String(err))
      outputCsv = null
    }
  }
</script>

<div class="container">
  <aside>
    <div class="header">
      <button on:click={() => (newSourceName = '')}>New Source</button>
    </div>
    <div class="files">
      {#each project.sources as source, i}
        <div class="file" class:active={i === sourceIndex} on:click={() => (sourceIndex = i)}>
          {source.name}
        </div>
      {/each}
    </div>
  </aside>
  <main>
    {#if project.sources[sourceIndex]}
      <button on:click={addFilesDialog}>Add files</button>
      <div class="options">
        <Options source={project.sources[sourceIndex]} />
      </div>
    {/if}
    <div class="output-header">
      <h3>Output</h3>
      <button on:click={generate}>Generate</button>
      {#if outputCsv !== null}
        <button on:click={csvCopy}>Copy</button>
        <button on:click={csvSaveAs}>Save As...</button>
      {/if}
    </div>
    <div class="table">
      {#if generating}
        Generating...
      {:else if outputCsv !== null}
        <CsvTable csv={outputCsv} />
      {/if}
    </div>
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

<FileDrop extensions={['csv', 'tsv']} handleFiles={addFiles} let:files>
  {#if files.length > 0}
    <h1 class="dropzone" class:droppable={files.length > 0} transition:fade={{ duration: 80 }}>
      Drop files to add
    </h1>
  {/if}
</FileDrop>

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
  .files
    overflow-y: auto
    padding: 5px 0px
    height: 100%
    margin-bottom: 10px
    border: 1px solid #D1D7DD
    .file
      padding: 4px 10px
      font-size: 14px
      cursor: default
      &.active
        background-color: hsla(0, 0%, 100%, 0.1)
  main
    display: flex
    flex-direction: column
    width: 10px
    flex-grow: 1
  .options
    display: flex
    flex-shrink: 0
  .output-header
    display: flex
    flex-shrink: 0
  .table
    height: 20px
    width: 100%
    overflow: auto
    flex-grow: 1
  .dropzone
    position: fixed
    width: 100%
    height: 100%
    top: 0px
    left: 0px
    display: flex
    align-items: center
    justify-content: center
    background-color: rgba(#000000, 0.4)
    text-shadow: 0px 0px 30px #000000
</style>
