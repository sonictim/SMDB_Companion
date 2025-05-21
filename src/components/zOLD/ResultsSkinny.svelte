<!-- filepath: /Users/tfarrell/Documents/CODE/SMDB_Companion/src/components/ResultsSkinny.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import Table from "../results/Table.svelte";
  import Filters from "../results/filterSwitch.svelte";
  import RemoveBar from "../results/removeBar.svelte";
  import Toolbar from "../results/Toolbar.svelte";
  import RemoveButton from "../results/removeButton.svelte";
  import Status from "../Status.svelte";
  import { preferencesStore } from "../../stores/preferences";
  import { showStatus, searchProgressStore } from "../../stores/status";

  import { Loader } from "lucide-svelte";

  export let selectedDb: string | null = null;

  let processing = false;
  let loading = true;

  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let unlistenRemoveFn: () => void;

  onMount(async () => {
    loading = false;

    unlistenRemoveFn = await listen<{
      stage: string;
    }>("remove-status", (event) => {});
  });

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });
</script>

<div class="block" style="width: 75vw;">
  <div class="header">
    <Filters />
    <RemoveButton />
  </div>
  {#if $preferencesStore.showToolbars}
    <Toolbar />
  {/if}
  <div class="block inner" style="margin-bottom: 15px;">
    {#if $showStatus}
      <Status />
    {:else if loading}
      <p class="ellipsis">Loading data...</p>
    {:else if processing}
      <div class="block inner">
        <span>
          <Loader
            size={24}
            class="spinner ml-2"
            style="color: var(--accent-color)"
          />
          {removeMessage}
        </span>
        <div class="progress-container">
          <div class="progress-bar" style="width: {removeProgress}%"></div>
        </div>
      </div>
    {:else}
      <Table />
    {/if}
  </div>
  {#if $preferencesStore.showToolbars}
    <RemoveBar />
  {/if}
</div>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    background-color: var(--secondary-bg);
    margin-bottom: 10px;
  }
</style>
