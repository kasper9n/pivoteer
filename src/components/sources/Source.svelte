<script lang="ts">
  import { popup } from '../../scripts/helpers'
  import FileDrop from 'svelte-tauri-filedrop'
  import { dialog } from '@tauri-apps/api'
  import { Source, sourceTypes } from '../../bindings'
  import { fade } from 'svelte/transition'

  export let source: Source

  async function addFiles(files: string[]) {
    for (const file of files) {
      if (source.files.includes(file)) {
        await popup('Skipping duplicate file: ' + file)
      } else {
        source.files.push(file)
        source.files = source.files
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
</script>

<section class="my-10">
  <h2>Source</h2>
  <div class="row">
    <p>Name:</p>
    <input type="text" bind:value={source.name} />
  </div>
  <div class="row">
    <p>Type:</p>
    <select bind:value={source.source_type}>
      {#each sourceTypes as sourceType}
        <option value={sourceType.type}>{sourceType.name}</option>
      {/each}
    </select>
  </div>
</section>
<section class="files-section">
  <div class="row">
    <h3>Files</h3>
    <button on:click={addFilesDialog}>Add files</button>
  </div>
  <div class="files">
    {#each source.files as file}
      <p>{file.replace(/^.*[\\/]/, '')}</p>
    {/each}
  </div>
</section>

<FileDrop extensions={['csv', 'tsv']} handleFiles={addFiles} let:files>
  {#if files.length > 0}
    <h1 class="dropzone" class:droppable={files.length > 0} transition:fade={{ duration: 80 }}>
      Drop files to add
    </h1>
  {/if}
</FileDrop>

<style lang="sass">
  .my-10
    margin-top: 10px
    margin-bottom: 10px
  .files
    height: 0px
    flex-grow: 1
    display: flex
    flex-direction: column
    overflow: auto
  .files-section
    flex-grow: 1
    height: 0px
    display: flex
    flex-direction: column
  p
    margin: 0px
  .row
    display: flex
    align-items: center
  input
    margin-left: 5px
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
