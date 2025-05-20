<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import Table from "./results/common/Table.svelte";
  import { preferencesStore } from "../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    clearSelected,
    invertSelected,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
    totalChecksStore,
    selectedChecksStore,
    updateCurrentFilter,
    filtersStore,
  } from "../stores/results";
  import { removeRecords, removeSelectedRecords } from "../stores/remove";
  import {
    metadataStore,
    replaceMetadata,
    toggleMarkDirty,
  } from "../stores/metadata";
  import { isRemove } from "../stores/menu";
  import { algoEnabled } from "../stores/algorithms";

  export let selectedDb: string | null = null;

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: currentFilter = $currentFilterStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;

  let processing = false;
  let loading = true;

  let processingBatch = false;
  let loadingResults = true;
  let showLoadingOverlay = true;

  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    updateCurrentFilter(select.value);
  }

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
    setTimeout(() => {
      loading = false;
    }, 100);

    activateResultsTab();
  });

  $: filters = $filtersStore;

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

  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    TriangleAlert,
    Loader,
  } from "lucide-svelte";

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    setTimeout(() => {
      try {
        checkSelected();
      } finally {
        processingBatch = false;
      }
    }, 10);
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

    <div style="margin-left: auto; display: flex; gap: 20px;">
      {#if $isRemove}
        {#if selectedItems.size > 0}
          <button class="cta-button cancel" on:click={removeSelectedRecords}>
            <OctagonX size="18" />
            Remove {selectedChecks} Selected Records
          </button>
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove all {totalChecks} Records
          </button>
        {:else}
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove {totalChecks} Records
          </button>
        {/if}
      {:else}
        <button class="cta-button cancel" on:click={replaceMetadata}>
          <NotebookPenIcon size="18" />
          <span>Replace '{metadata.find}' with '{metadata?.replace || ""}'</span
          >
        </button>
      {/if}
    </div>
  </div>
  {#if $preferencesStore.showToolbars}
    <div
      class="bar"
      style="margin-top: 10px; margin-bottom: 20px; padding: 0px;"
    >
      {#if enableSelections}
        <button class="small-button" on:click={toggleChecksSelected}
          >Toggle Selected</button
        >
        <button class="small-button" on:click={checkSelectedWithIndicator}
          >Check Selected</button
        >
        <button class="small-button" on:click={uncheckSelected}
          >Uncheck Selected</button
        >
        <button class="small-button" on:click={invertSelected}
          >Invert Selections</button
        >
        <button class="small-button" on:click={clearSelected}
          >Clear Selections</button
        >
        {#if selectedItems.size > 0}
          <p style="margin-left: 10px">({selectedItems.size} selected)</p>
        {/if}
        {#if processingBatch}
          <div class="batch-processing">
            <Loader size={24} class="spinner" />
            <span>Processing {selectedItems.size} items...</span>
          </div>
        {/if}
      {/if}

      <div class="filter-container">
        {#if $isRemove}
          <span>Filter by: </span>
          <select
            class="select-field"
            bind:value={currentFilter}
            on:change={handleFilterChange}
          >
            {#each filters as option}
              {#if option.enabled}
                {#if option.id === "spacer"}
                  <option disabled>{option.name}</option>
                {:else}
                  <option value={option.id}>{option.name}</option>
                {/if}
              {/if}
            {/each}
          </select>
        {:else}
          <button
            type="button"
            class="grid item"
            style="margin-left: 120px"
            on:click={toggleMarkDirty}
          >
            {#if $metadataStore.mark_dirty}
              <CheckSquare
                size={20}
                class="checkbox checked {metadata.column == 'FilePath' ||
                metadata.column == 'Filename' ||
                metadata.column == 'Pathname'
                  ? 'inactive'
                  : ''}"
              />
            {:else}
              <Square size={20} class="checkbox" />
            {/if}
            <span
              class={metadata.column == "FilePath" ||
              metadata.column == "Filename" ||
              metadata.column == "Pathname"
                ? "inactive"
                : ""}>Mark Records as Dirty</span
            >
          </button>
        {/if}
      </div>
    </div>
  {/if}
  <div class="block inner" style="margin-bottom: 15px;">
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
    {:else}
      <Table />
    {/if}
  </div>
  {#if $preferencesStore.showToolbars}
    <div class="header" style="margin-bottom: 0px; margin-top: 0px;">
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

  .header h2 {
    margin: 0;
  }

  .spinner {
    animation: spin 1.5s linear infinite;
    color: var(--accent-color);
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
