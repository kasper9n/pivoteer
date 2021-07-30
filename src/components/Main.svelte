<script lang="ts">
  import { dialog } from '@tauri-apps/api'
  import { popup } from '../scripts/helpers'
  import type { Instance } from 'src/scripts/instance'
  import FileDrop from './FileDrop.svelte'

  export let instance: Instance
  const allowedFileExtensions = ['csv']

  async function addCSVs() {
    const filePaths = await dialog.open({
      filters: [{ name: 'CSV', extensions: allowedFileExtensions }],
      multiple: true,
    })
    if (filePaths instanceof Array) {
      for (const path of filePaths) {
        if (instance.files.includes(path)) {
          await popup('Skipping duplicate file: ' + path)
        } else {
          instance.files.push(path)
          instance.files = instance.files
        }
      }
    }
  }
  function handleFiles(files: string[]) {
    for (const file of files) {
      instance.files.push(file)
      instance.files = instance.files
    }
  }
</script>

<div class="area">
  <div class="header">
    <button on:click={addCSVs}>Add CSVs</button>
  </div>
  <div class="files">
    {#each instance.files as file}
      <div class="file">
        {file.replace(/^.*[\\\/]/, '')}
      </div>
    {/each}
  </div>
</div>

<FileDrop {allowedFileExtensions} {handleFiles} />

<style lang="sass">
  .area
    min-width: 300px
    display: flex
    flex-direction: column
    width: 20%
    height: 100%
    float: left
    background-color: rgba(#FFFFFF, 0.05)
  .header
    padding: 10px
    background-color: rgba(#FFFFFF, 0.05)
  .files
    overflow-y: auto
    .file
      padding: 4px 10px
      font-size: 14px
</style>
