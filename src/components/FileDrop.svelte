<script lang="ts">
  import { fade } from 'svelte/transition'
  import { extractUnlistener } from '../scripts/helpers'
  import { event } from '@tauri-apps/api'
  import { onMount } from 'svelte'

  let droppable = false
  export let allowedFileExtensions: string[] = []
  export let handleFiles: (files: string[]) => void

  // workaround for https://github.com/tauri-apps/tauri/issues/2323
  let readyToListen = false
  setTimeout(() => (readyToListen = true), 100)

  function getValidPaths(paths: string[]) {
    let validPaths = []
    for (const path of paths) {
      for (const ext of allowedFileExtensions) {
        if (path.endsWith('.' + ext)) {
          validPaths.push(path)
        }
      }
    }
    return validPaths
  }
  onMount(() => {
    const unlisten = event.listen('tauri://file-drop', (e) => {
      if (!readyToListen) return
      const validPaths = getValidPaths(e.payload as string[])
      if (validPaths.length > 0) {
        // https://github.com/tauri-apps/tauri/pull/2300
        droppable = true
      }
      console.log('Hover', e)
    })
    return extractUnlistener(unlisten)
  })
  onMount(() => {
    const unlisten = event.listen('tauri://file-drop-hover', (e) => {
      if (!readyToListen) return
      const validPaths = getValidPaths(e.payload as string[])
      if (validPaths.length > 0) {
        // https://github.com/tauri-apps/tauri/pull/2300
        handleFiles(validPaths)
        droppable = false
      }
      console.log('Drop', e)
    })
    return extractUnlistener(unlisten)
  })
  onMount(() => {
    const unlisten = event.listen('tauri://file-drop-cancelled', (e) => {
      if (!readyToListen) return
      droppable = false
      console.log('Cancel', e)
    })
    return extractUnlistener(unlisten)
  })
</script>

{#if droppable}
  <!-- if the overlay is always visible, it's not possible to scroll while dragging tracks -->
  <div class="drag-overlay" transition:fade={{ duration: 100 }}>
    <h1>Drop files to import</h1>
  </div>
  <div class="dropzone" />
{/if}

<style lang="sass">
  .dropzone, .drag-overlay
    position: fixed
    width: 100%
    height: 100%
    top: 0px
    left: 0px
  .drag-overlay
    display: flex
    align-items: center
    justify-content: center
    background-color: rgba(#10161e, 0.9)
    transition: all 100ms ease-in-out
</style>
