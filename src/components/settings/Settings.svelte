<script lang="ts">
  import type { Project } from '../../scripts/project'

  export let project: Project
</script>

Action:
<select bind:value={column.action}>
  <option value="Unique">Keep unique values</option>
  <option value="Sum">Sum</option>
</select>
