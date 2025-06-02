<script lang="ts">
  // Svelte lifecycle imports
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  // Component imports
  import Table from "./results/Table.svelte";
  import Filters from "./results/filterSwitch.svelte";
  import RemoveBar from "./results/removeBar.svelte";
  import Toolbar from "./results/Toolbar.svelte";
  import RemoveButton from "./results/removeButton.svelte";
  import Status from "./Status.svelte";
  import Form from "./registration/Form.svelte";

  // Icon imports
  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    TriangleAlert,
    Loader,
  } from "lucide-svelte";

  // Store imports
  import { preferencesStore } from "../stores/preferences";
  import { isRegistered } from "../stores/registration";
  import { showStatus, searchProgressStore } from "../stores/status";
  import { isRemove } from "../stores/menu";
  import {
    resultsStore,
    filteredItemsStore,
    totalChecksStore,
  } from "../stores/results";

  // Store subscriptions
  $: results = $resultsStore;
  $: filteredItems = $filteredItemsStore;

  $: totalChecks = $totalChecksStore;

  // Calculate total count across all groups
  $: totalResultsCount = results.reduce((sum, group) => sum + group.length, 0);

  // UI state variables
  let processing = false;
  let loading = true;
  let loadingResults = true;
  let showLoadingOverlay = true;

  // Remove process state
  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let removeStage = "";
  let unlistenRemoveFn: () => void;

  // Data loading functions
  async function fetchData() {
    try {
      loading = true;
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  function activateResultsTab() {
    loadingResults = true;
    showLoadingOverlay = true;

    setTimeout(() => {
      const timer = setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);

      return () => clearTimeout(timer);
    }, 100);
  }

  onMount(async () => {
    setTimeout(() => {
      loading = false;
    }, 100);

    activateResultsTab();

    unlistenRemoveFn = await listen<{
      progress: number;
      message: string;
      stage: string;
    }>("remove-status", (event) => {
      const status = event.payload;
      removeProgress = status.progress;
      removeMessage = status.message;
      removeStage = status.stage;
      console.log(
        `Remove status: ${status.stage} - ${status.progress}% - ${status.message}`
      );
      if (status.stage === "complete") {
        processing = false;
        fetchData();
      }
    });
  });

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });

  $: {
    if (filteredItems && filteredItems.length > 0 && loadingResults) {
      setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);
    }
  }
</script>

<div class="block results-container">
  <div class="header">
    <h2>Search Results:</h2>
    <span style="font-size: var(--font-size-lg)">
      {#if $isRemove}
        {totalChecks} of {totalResultsCount} Records marked for Removal
      {:else}
        {totalResultsCount} Records found
      {/if}
    </span>
    <RemoveButton />
  </div>
  {#if $isRegistered}
    {#if $preferencesStore.showToolbars}
      <span>
        <Toolbar>
          <div slot="right">
            <Filters />
          </div>
        </Toolbar>
      </span>
    {/if}
    <div class="block inner results-content">
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
  {:else}
    <p>Registration Required to View Results</p>
    <Form />
  {/if}
</div>

<style>
  .ellipsis {
    border-radius: 5px;
    animation: loading 1s infinite;
  }

  @keyframes loading {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
    100% {
      opacity: 1;
    }
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    background-color: var(--secondary-bg);
    margin-bottom: 10px;
  }

  .header h2 {
    margin: 0;
  }

  .results-container {
    display: flex;
    flex-direction: column;
    height: calc(
      100vh - (var(--font-size) * 3)
    ); /* Adjust the 60px based on your header height */
  }

  .results-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 300px; /* Minimum height to ensure it's always visible */
  }

  /* This ensures the Table component knows it should fill the space */
  .results-content :global(.virtualized-table-container) {
    flex: 1;
    height: 100% !important;
    min-height: 250px;
  }
</style>
