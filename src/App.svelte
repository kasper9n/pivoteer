<script lang="ts">
  // import * as tauri from '@tauri-apps/api/tauri'
  import { event } from '@tauri-apps/api'
  import { popup } from './scripts/helpers'
  import { onDestroy } from 'svelte'
  import type { Instance } from './scripts/instance'
  import { getDefaultInstance } from './scripts/instance'
  import Main from './components/Main.svelte'

  let currentInstance: Instance | null = null

  async function newProject() {
    currentInstance = getDefaultInstance()
  }

  const unlistenFuture = event.listen('menu', ({ payload }) => {
    if (payload === 'New') {
      newProject().catch(popup)
    }
  })
  onDestroy(async () => {
    const unlisten = await unlistenFuture
    unlisten()
  })
</script>

{#if currentInstance}
  <Main instance={currentInstance} />
{:else}
  <div class="start-page">
    <button on:click={newProject}>New Instance</button>
  </div>
{/if}

<style lang="sass">
  :global(html)
    height: 100%
    box-sizing: border-box
    overflow: hidden
  :global(body)
    margin: 0
    font-family: Arial, Helvetica, sans-serif
    font-size: 18px
    background-color: #191B20
    color: white
    height: 100%
  .start-page
    display: flex
    height: 100%
    align-items: center
    justify-content: center
</style>
