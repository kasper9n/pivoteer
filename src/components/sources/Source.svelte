<script lang="ts">
  import { popup } from '../../scripts/helpers'
  import FileDrop from 'svelte-tauri-filedrop'
  import { dialog } from '@tauri-apps/api'
  import type { Source, SourceType } from '../../bindings'
  import { fade } from 'svelte/transition'
  import ColumnConfig from './ColumnConfig.svelte'

  export let source: Source

  function kindOptions(): { kind: SourceType; text: string }[] {
    return [
      { text: 'Bandcamp', kind: { id: 'Bandcamp' } },
      { text: 'Landr', kind: { id: 'Landr' } },
      { text: 'Pretzel', kind: { id: 'Pretzel' } },
      { text: 'Repost By SoundCloud', kind: { id: 'RepostBySoundCloud' } },
      { text: 'Stem', kind: { id: 'Stem' } },
      { text: 'Symphonic', kind: { id: 'Symphonic' } },
      {
        text: 'Custom',
        kind: {
          id: 'Custom',
          content: {
            header_row_index: 0,
            isrc: null,
            upc: null,
            revenue: null,
          },
        },
      },
    ]
  }
  let kindIndex: number
  $: getKindIndex(source)
  $: setKind(kindIndex)
  function getKindIndex(source: Source) {
    kindIndex = kindOptions().findIndex((o) => o.kind.id === source.kind.id)
  }
  function setKind(index: number) {
    source.kind = kindOptions()[index].kind
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

<section class="my-10">
  <h2>Source</h2>
  <div class="row">
    <p>Name:</p>
    <input type="text" bind:value={source.name} />
  </div>
  <div class="row">
    <p>Type:</p>
    <select bind:value={kindIndex}>
      {#each kindOptions() as option, i}
        <option value={i}>{option.text}</option>
      {/each}
    </select>
  </div>
  <div class="ml">
    {#if source.kind.id === 'Custom'}
      <div class="row">
        <p>Header row index:</p>
        <input type="number" bind:value={source.kind.content.header_row_index} />
      </div>
      <ColumnConfig label="ISRC" bind:column={source.kind.content.isrc} />
      <ColumnConfig label="UPC" bind:column={source.kind.content.upc} />
      <ColumnConfig label="Revenue" bind:column={source.kind.content.revenue} />
    {/if}
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
  .ml
    margin-left: 40px
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
