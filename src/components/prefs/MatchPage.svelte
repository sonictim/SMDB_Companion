<script lang="ts">
  import Matches from "./Match.svelte";
  import SafeFolders from "./SafeFolders.svelte";
  import WaveFormSearch from "./WaveFormSearch.svelte";

  // Use the store directly instead of assigning to `pref`
</script>

<div class="grid-container">
  <div class="page-columns">
    <Matches />
    <SafeFolders />
  </div>
  <WaveFormSearch />
</div>

<style>
  .page-columns {
    display: grid;
    grid-template-columns: 1fr 1fr; /* Left, center, right */
    height: 100%;
    /* gap: 10px; */
  }

  .grid-container {
    height: 100%;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 2fr 1fr;
    /* gap: 10px; */
    margin-bottom: 0px;
  }
</style>
