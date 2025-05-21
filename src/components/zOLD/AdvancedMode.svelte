<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Database, Search as SearchIcon, X } from "lucide-svelte";
  import { basename, extname } from "@tauri-apps/api/path";
  import {
    databaseStore,
    openDatabase,
    getCompareDb,
  } from "../../stores/database";
  import { preferencesStore } from "../../stores/preferences";
  import {
    toggleAlgorithm,
    getAlgorithmTooltip,
  } from "../../stores/algorithms";
  import {
    resultsStore,
    selectedItemsStore,
    totalChecksStore,
    selectedChecksStore, // Import the new store
  } from "../../stores/results";
  import {
    searchProgressStore,
    showStatus,
    initializeSearchListeners,
    toggleSearch,
  } from "../../stores/status";

  import Table from "../results/Table.svelte";
  import { removeRecords, removeSelectedRecords } from "../../stores/remove";

  export let selectedDb: string | null = null;

  $: selectedItems = $selectedItemsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore; // Create a reactive reference to selected checks
  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  let processing = false;
  let loading = true;

  async function fetchData() {
    try {
      loading = true;
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loading = false;
    initializeSearchListeners();
    fetchData();
  });

  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let removeStage = "";
  let unlistenRemoveFn: () => void;

  onMount(async () => {
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

  import { CheckSquare, Square, OctagonX, Loader } from "lucide-svelte";

  function getAlgoClass(algo: { id: string }, algorithms: any[]) {
    if (
      (algo.id === "audiosuite" || algo.id === "filename") &&
      !algorithms.find((a) => a.id === "basic")?.enabled
    ) {
      return "inactive";
    }
    return "";
  }

  async function getFilenameWithoutExtension(fullPath: string) {
    const name = await basename(fullPath); // Extracts filename with extension
    const ext = await extname(fullPath); // Extracts extension
    return name.replace(ext, ""); // Removes extension
  }
</script>

<div class="top-bar">
  <div class="top-bar-left">
    <button class="nav-link" on:click={() => openDatabase(false)}>
      <Database size={18} />
      <span style="font-size: 24px;">
        {$databaseStore?.name || "Select Database"}
        {#if $databaseStore}
          <span style="font-size: 14px;"
            >{$databaseStore.size} total records</span
          >
        {/if}
        {#if selectedItems.size > 0}
          <span style="font-size: 14px;">({selectedItems.size} selected)</span>
        {/if}
      </span>
    </button>
  </div>
  <div class="top-bar-right">
    {#if $databaseStore}
      <button
        class="nav-link"
        on:click={async () => {
          toggleSearch();
        }}
        title="Search for Duplicates"
      >
        <div class="flex items-center gap-2">
          {#if $showStatus}
            <X size={18} />
            <span>Cancel Search</span>
          {:else}
            <SearchIcon size={18} />
            <span>Search for Records</span>
          {/if}
        </div>
      </button>
    {/if}
    {#if $resultsStore.length > 0}
      <button
        class="nav-link"
        on:click={() => {
          if (selectedItems.size > 0) removeSelectedRecords();
          else removeRecords();
        }}
        title="Remove Duplicates"
      >
        <div class="flex items-center gap-2">
          <OctagonX size={18} />
          {#if selectedItems.size > 0}
            Remove {selectedChecks} Selected Records
          {:else}
            <span>Remove {totalChecks} Records</span>
          {/if}
        </div>
      </button>
    {/if}
  </div>
</div>

<div class="block">
  {#if loading}
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
  {:else if $resultsStore.length > 0}
    <Table />
  {:else if $showStatus}
    <div class="block inner">
      <span>
        <Loader
          size={24}
          class="spinner ml-2"
          style="color: var(--accent-color)"
        />
        {$searchProgressStore.searchMessage}
      </span>
      <div class="progress-container">
        <div
          class="progress-bar"
          style="width: {$searchProgressStore.searchProgress}%"
        ></div>
      </div>
      <span>
        {$searchProgressStore.subsearchMessage}
      </span>
      <div class="progress-container">
        <div
          class="progress-bar"
          style="width: {$searchProgressStore.subsearchProgress}%"
        ></div>
      </div>
    </div>
  {:else}
    <div class="grid">
      {#each $preferencesStore.algorithms as algo}
        <div
          class="grid item {getAlgoClass(algo, $preferencesStore.algorithms)}"
        >
          <button
            type="button"
            class="grid item"
            on:click={() => toggleAlgorithm(algo.id)}
          >
            {#if algo.id === "audiosuite" || algo.id === "filename"}
              <span style="margin-right: 20px;"></span>
            {/if}

            {#if algo.enabled}
              <CheckSquare
                size={20}
                class="checkbox {(algo.id === 'audiosuite' ||
                  algo.id === 'filename') &&
                !isBasicEnabled
                  ? 'inactive'
                  : 'checked'}"
              />
            {:else}
              <Square size={20} class="checkbox inactive" />
            {/if}

            <span
              class="tooltip-trigger {(algo.id === 'audiosuite' ||
                algo.id === 'filename') &&
              !isBasicEnabled
                ? 'inactive'
                : ''}"
            >
              {algo.name}
              <span class="tooltip-text">{getAlgorithmTooltip(algo.id)} </span>
            </span>
          </button>

          {#if algo.id === "dbcompare"}
            {#if algo.db !== null && algo.db !== undefined}
              {#await getFilenameWithoutExtension(algo.db) then filename}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span class="clickable" on:click={getCompareDb}>{filename}</span
                >
              {/await}
            {:else}
              <button
                type="button"
                class="small-button"
                style="border-color: var(--secondary-bg)"
                on:click={getCompareDb}>Select DB</button
              >
            {/if}
          {/if}

          {#if algo.id === "duration"}
            <input
              type="number"
              min="0"
              step="0.1"
              bind:value={algo.min_dur}
              class="duration-input"
              style="width: 55px; background-color: var(--primary-bg)"
            />
            s
          {/if}
        </div>
      {/each}
    </div>
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

  .list-item {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
    width: max(var(--total-width), 100vw);
  }

  .algorithm-icons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .icon-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon-wrapper:hover::after {
    content: attr(title);
    position: absolute;
    background: var(--primary-bg);
    border: 1px solid var(--border-color);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    z-index: 100;
    white-space: nowrap;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
  }

  /* Make BOTH header and cells sticky */
  .sticky-column {
    position: sticky !important;
    left: 0;
    z-index: 15;
    background-color: var(
      --primary-bg
    ); /* Background prevents content behind from showing through */
    box-shadow: 1px 0 3px rgba(0, 0, 0, 0.1); /* Optional shadow for depth */
  }
  .sticky-column-right {
    position: sticky !important;
    right: 0;
    z-index: 15;
    background-color: var(
      --primary-bg
    ); /* Background prevents content behind from showing through */
    box-shadow: 1px 0 3px rgba(0, 0, 0, 0.1); /* Optional shadow for depth */
    text-align: right;
  }

  /* Add these styles to preserve highlighting on sticky columns */
  .grid-item.sticky-column.selected,
  .grid-item.sticky-column-right.selected {
    background-color: var(--accent-color) !important;
  }

  .select-field {
    flex-grow: 0;
  }

  .checked-item {
    color: var(--warning-hover);
    background-color: var(--primary-bg);
  }
  /* font-weight: bold; */
  .block {
    background-color: var(--secondary-bg);
    padding: 0px 0px;
    /* padding-bottom: 20px; */
    border-radius: 8px;
    flex: 1;
    /* Allow features to grow and shrink */
    margin-bottom: 0px;
    /* margin-top: -20px; */
    /* Adjust as needed */
    display: flex;
    flex-direction: column;
    /* Stack items vertically */
    height: calc(100vh - 55px);
    /* Full viewport height */
  }

  .block.inner {
    background-color: var(--primary-bg);
    padding: 0px;
    border-radius: 6px;
    /* display: flex; */
    /* flex-direction: column; */
    flex-grow: 1;
    /* height: 50vh; */
    /* height: calc(100vh - 240px); */
    overflow-y: auto;
  }

  .grid {
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, auto);
    grid-auto-flow: column;
  }
</style>
