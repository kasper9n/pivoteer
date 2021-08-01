<script lang="ts">
  import { dialog, invoke, fs } from '@tauri-apps/api'
  import * as clipboard from '@tauri-apps/api/clipboard'
  import { popup } from '../scripts/helpers'
  import type { Project } from '../scripts/project'
  import CsvTable from './CsvTable.svelte'
  import FileDrop from './FileDrop.svelte'
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
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      multiple: true,
    })
    if (filePaths instanceof Array) {
      addFiles(filePaths)
    }
  }
  let csv: string | null = null
  async function generate() {
    try {
      csv = await invoke('generate', { project })
    } catch (err) {
      popup(err)
      csv = null
    }
  }
  function csvCopy() {
    if (csv === null) return
    clipboard.writeText(csv)
  }
  async function csvSaveAs() {
    try {
      if (csv === null) return
      const filePath = await dialog.save({
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      })
      if (!filePath) return
      await fs.writeFile({
        path: filePath,
        contents: csv,
      })
    } catch (err) {
      popup(err)
      csv = null
    }
  }
</script>

<div class="container">
  <div class="sidebar">
    <div class="header">
      <button on:click={addFilesDialog}>Add CSVs</button>
    </div>
    <div class="files">
      {#each project.files as file}
        <div class="file">
          {file.replace(/^.*[\\\/]/, '')}
        </div>
      {/each}
    </div>
  </div>
  <main>
    <Options {project} />
    <div class="output-header">
      <h3>Output</h3>
      <button on:click={generate}>Generate</button>
      {#if csv !== null}
        <button on:click={csvCopy}>Copy</button>
        <button on:click={csvSaveAs}>Save As...</button>
      {/if}
    </div>
    {#if csv !== null}
      <CsvTable {csv} />
    {/if}
  </main>
</div>

<FileDrop fileExtensions={['csv']} handleFiles={addFiles} />

<style lang="sass">
  .container
    height: 100%
    display: flex
  .sidebar
    min-width: 250px
    display: flex
    flex-direction: column
    height: 100%
    float: left
  .header
    padding: 10px
    background-color: #303237
  .files
    overflow-y: auto
    padding: 5px 0px
    height: 100%
    background-color: #25272B
    .file
      padding: 4px 10px
      font-size: 14px
  main
    width: 100%
    padding: 10px
  .output-header
    display: flex
</style>
