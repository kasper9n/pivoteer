<script>
  import * as tauri from 'tauri/api/tauri'
  import * as dialog from 'tauri/api/dialog'

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
</script>

<style lang="sass">
  main
    text-align: center
    padding: 1em
    max-width: 240px
    margin: 0 auto
  :global(body)
    margin: 0
    font-family: Arial, Helvetica, sans-serif
    font-size: 18px
  @media (min-width: 640px)
    main
      max-width: none
</style>

<main>
  <h1>Riddle</h1>
  <button on:click={importCsv}>Import</button>
  <p>
    Visit the <a href="https://svelte.dev/tutorial" target="_blank">Svelte tutorial</a> to learn how
    to build Svelte apps.
  </p>
</main>
