<script lang="ts">
  // import * as tauri from '@tauri-apps/api/tauri'
  import { invoke } from '@tauri-apps/api/tauri'
  import * as dialog from '@tauri-apps/api/dialog'
  import { emit, listen } from '@tauri-apps/api/event'

  let textbox = ''
  async function importCsv() {
    const filePath = await dialog.open({
      filters: [{ name: 'CSV', extensions: ['csv'] }],
      multiple: false,
    })
    try {
      textbox = await invoke('import_csv', {
        filePath: filePath,
      })
    } catch (e) {
      console.error(e)
    }
  }

  let textarea: HTMLTextAreaElement
  function selectAll() {
    textarea.select()
  }
</script>

<main>
  <div class="buttons">
    <button on:click={importCsv}>Import</button>
    <button on:click={selectAll}>Select all</button>
  </div>
  <textarea value={textbox} bind:this={textarea} />
</main>

<style lang="sass">
  main
    text-align: center
    padding: 30px
    padding-top: 10px
    margin: 0 auto
    height: 100%
    box-sizing: border-box
    display: flex
    flex-direction: column
  :global(html)
    height: 100%
    box-sizing: border-box
    overflow: hidden
  :global(body)
    margin: 0
    font-family: Arial, Helvetica, sans-serif
    font-size: 18px
    background-color: #1B1B1B
    color: white
    height: 100%
  textarea
    display: block
    margin-top: 10px
    width: 100%
    height: 100%
    resize: none
    box-sizing: border-box
</style>
