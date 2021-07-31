<script lang="ts">
  import { dialog } from '@tauri-apps/api'
  import { popup } from '../scripts/helpers'
  import type { Instance } from 'src/scripts/instance'
  import FileDrop from './FileDrop.svelte'
  import Options from './Options.svelte'

  export let instance: Instance
  const allowedFileExtensions = ['csv']

  async function addFiles(files: string[]) {
    for (const file of files) {
      if (instance.files.includes(file)) {
        await popup('Skipping duplicate file: ' + file)
      } else {
        instance.files.push(file)
        instance.files = instance.files
      }
    }
  }
  async function addFilesDialog() {
    const filePaths = await dialog.open({
      filters: [{ name: 'CSV', extensions: allowedFileExtensions }],
      multiple: true,
    })
    if (filePaths instanceof Array) {
      addFiles(filePaths)
    }
  }
</script>

<div class="container">
  <div class="sidebar">
    <div class="header">
      <button on:click={addFilesDialog}>Add CSVs</button>
    </div>
    <div class="files">
      {#each instance.files as file}
        <div class="file">
          {file.replace(/^.*[\\\/]/, '')}
        </div>
      {/each}
    </div>
  </div>
  <Options {instance} />
</div>

<FileDrop {allowedFileExtensions} handleFiles={addFiles} />

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
</style>
