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

<div class="block">
  <div class="header">
    <h2>Search Results:</h2>
    <span style="font-size: 18px">
      {#if $isRemove}
        {totalChecks} of {results.length} Records marked for Removal
      {:else}
        {results.length} Records found
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
      <!-- <div class="header" style="margin-bottom: 0px; margin-top: 0px;">
      {#if $isRemove}
        <span>
          Remove Records from:
          <select
            class="select-field"
            bind:value={pref.safety_db}
            on:change={() => preferencesStore.set(pref)}
          >
            {#each [{ bool: true, text: "Safety Database Copy" }, { bool: false, text: "Current Database" }] as option}
              <option value={option.bool}>{option.text}</option>
            {/each}
          </select>
          {#if pref.safety_db}
            with tag:
            <input
              class="input-field"
              placeholder="thinned"
              type="text"
              id="new_db_tag"
              bind:value={pref.safety_db_tag}
              on:change={() => preferencesStore.set(pref)}
            />
          {:else}
            <TriangleAlert
              size="30"
              class="blinking"
              style="color: var(--warning-hover); margin-bottom: -10px"
            />
          {/if}
        </span>
        {#if algoEnabled("dual_mono")}
          <span>
            Dual Mono Files:
            <select
              class="select-field"
              bind:value={pref.strip_dual_mono}
              on:change={() => preferencesStore.set(pref)}
            >
              {#each [{ id: false, text: "Preserve" }, { id: true, text: "Strip" }] as option}
                <option value={option.id}>{option.text}</option>
              {/each}
            </select>
            {#if pref.strip_dual_mono}
              <TriangleAlert
                size="30"
                class="blinking"
                style="color: var(--warning-hover); margin-bottom: -10px"
              />
            {/if}
          </span>
        {/if}
        <span>
          Checked Files:
          <select
            class="select-field"
            bind:value={$preferencesStore.erase_files}
          >
            {#each [{ id: "Keep", text: "Keep on Disk" }, { id: "Trash", text: "Move To Trash" }, { id: "Delete", text: "Permanently Delete" }] as option}
              <option value={option.id}>{option.text}</option>
            {/each}
          </select>
          {#if pref.erase_files !== "Keep"}
            <TriangleAlert
              size="30"
              class={pref.erase_files == "Delete" ? "blinking" : ""}
              style="color: var(--warning-hover); margin-bottom: -10px"
            />
          {/if}
        </span>
      {/if}
    </div> -->
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
</style>
