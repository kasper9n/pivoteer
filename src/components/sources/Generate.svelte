<script lang="ts">
  import { dialog, invoke, fs } from '@tauri-apps/api'
  import type { Source } from '../../scripts/project'
  import * as clipboard from '@tauri-apps/api/clipboard'
  import { popup } from '../../scripts/helpers'
  import CsvTable from './CsvTable.svelte'

  export let source: Source

  let outputCsv: string | null = null
  let generating = false
  async function generate() {
    if (generating) {
      return
    }
    try {
      generating = true
      outputCsv = await invoke('generate', { source })
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

<div class="output-header">
  <h3>Output</h3>
</div>
<button on:click={generate}>Generate</button>
{#if outputCsv !== null}
  <button on:click={csvCopy}>Copy</button>
  <button on:click={csvSaveAs}>Save As...</button>
{/if}
<div class="table">
  {#if generating}
    Generating...
  {:else if outputCsv !== null}
    <CsvTable csv={outputCsv} />
  {/if}
</div>

<style lang="sass">
  .table
    height: 20px
    width: 100%
    overflow: auto
    flex-grow: 1
  .output-header
    display: flex
    flex-shrink: 0
</style>
