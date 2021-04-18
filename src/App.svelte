<script>
  import * as tauri from '@tauri-apps/api/tauri'
  import * as dialog from '@tauri-apps/api/dialog'
  import { emit, listen } from '@tauri-apps/api/event'

  async function importCsv() {
    const filePath = await dialog.open({
      filter: 'csv',
      multiple: false,
    })
    tauri.invoke({
      cmd: 'importCsv',
      file_path: filePath,
    })
  }

  let output = ''
  listen('output', (e) => {
    output = e.payload
  })

  let textarea
  function selectAll(e) {
    textarea.select()
  }
</script>

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

<main>
  <div class="buttons">
    <button on:click={importCsv}>Import</button>
    <button on:click={selectAll}>Select all</button>
  </div>
  <textarea value={output} bind:this={textarea} />
</main>
