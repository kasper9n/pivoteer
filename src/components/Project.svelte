<script lang="ts">
  import { dialog, invoke, fs } from '@tauri-apps/api'
  import * as clipboard from '@tauri-apps/api/clipboard'
  import { popup } from '../scripts/helpers'
  import type { Project } from '../scripts/project'
  import CsvTable from './CsvTable.svelte'
  import Options from './Options.svelte'

  export let project: Project

  async function addFiles(files: string[]) {
    for (const file of files) {
      if (project.files.includes(file)) {
        await popup('Skipping duplicate file: ' + file)
      } else {
        project.files.push(file)
        project.files = project.files
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
    if (generating) return
    try {
      generating = true
      outputCsv = await invoke('generate', { project })
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
  <div class="sidebar">
    <div class="header">
      <button on:click={addFilesDialog}>Add Files</button>
    </div>
    <div class="files">
      {#each project.files as file}
        <div class="file">
          {file.replace(/^.*[\\/]/, '')}
        </div>
      {/each}
    </div>
  </div>
  <main>
    <div class="options">
      <Options {project} />
    </div>
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

<style lang="sass">
  .container
    height: 100%
    display: flex
  .sidebar
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
</style>
