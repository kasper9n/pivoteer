<script lang="ts">
  import type { FilterConfig, FilterOperator } from 'bindings'
  import ColumnLocation from './ColumnLocation.svelte'

  export let filters: FilterConfig[]

  function add() {
    let defaultFilter: FilterConfig = {
      column: {
        Name: '',
      },
      operator: 'Is',
      value: '',
    }
    filters.push(defaultFilter)
    filters = filters
  }
  const filterOperators: [string, FilterOperator][] = [
    ['is', 'Is'],
    ['is not', 'IsNot'],
  ]

  function removeIndex(index: number) {
    filters.splice(index, 1)
    filters = filters
  }
</script>

<div class="row">
  <h3>Filters</h3>
  <button on:click={add}>+</button>
</div>
{#each filters as filter, i}
  <div class="row">
    <button on:click={() => removeIndex(i)}>-</button>
    <div>
      <div class="row">
        <ColumnLocation label="Column" bind:column={filter.column} />
      </div>
      <div class="row">
        <select bind:value={filter.operator}>
          {#each filterOperators as filterOperator}
            <option value={filterOperator[1]}>{filterOperator[0]}</option>
          {/each}
        </select>
        <input type="text" bind:value={filter.value} />
      </div>
    </div>
  </div>
{/each}

<style lang="sass">
  .row
    display: flex
    align-items: center
</style>
