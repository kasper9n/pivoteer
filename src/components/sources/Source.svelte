<script lang="ts">
  import { popup } from '../../scripts/helpers'
  import FileDrop from 'svelte-tauri-filedrop'
  import { dialog } from '@tauri-apps/api'
  import type { Source } from '../../bindings'
  import ColumnOption from './ColumnOption.svelte'
  import { fade } from 'svelte/transition'

  export let source: Source
  let headerRowNumber: number | null = 1
  $: {
    if (headerRowNumber === null) {
      source.headerRowIndex = 1
    } else {
      if (headerRowNumber < 1) {
        headerRowNumber = 1
      } else if (headerRowNumber % 1 !== 0) {
        headerRowNumber = Math.floor(headerRowNumber)
      }
      source.headerRowIndex = headerRowNumber - 1
    }
  }
  function addColumn() {
    source.columns.push({
      idType: 'Name',
      id: '',
    })
    source.columns = source.columns
  }
  function removeColumn(index: number) {
    source.columns.splice(index, 1)
    source.columns = source.columns
  }

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

<section>
  <h2>Source</h2>
</section>
<section>
  <div class="row">
    <p>Name:</p>
    <input type="text" bind:value={source.name} />
  </div>
  <div class="row">
    <p>Header row number:</p>
    <input type="number" bind:value={headerRowNumber} placeholder="1" />
  </div>
  <div class="row">
    <p>Columns</p>
    <button on:click={addColumn}>+</button>
  </div>
  {#each source.columns as column, i}
    <ColumnOption bind:column remove={() => removeColumn(i)} />
  {/each}
</section>
<section>
  <div class="row">
    <h3>Files</h3>
    <button on:click={addFilesDialog}>Add files</button>
  </div>
  {#each source.files as file}
    <p>{file.replace(/^.*[\\/]/, '')}</p>
  {/each}
</section>

<FileDrop extensions={['csv', 'tsv']} handleFiles={addFiles} let:files>
  {#if files.length > 0}
    <h1 class="dropzone" class:droppable={files.length > 0} transition:fade={{ duration: 80 }}>
      Drop files to add
    </h1>
  {/if}
</FileDrop>

<style lang="sass">
  section
    margin: 10px 0px
  p
    margin: 0px
  .row
    display: flex
    align-items: center
  input
    margin-left: 5px
  input[type="number"]
    margin-left: 5px
    width: 40px
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
