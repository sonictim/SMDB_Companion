<!-- filepath: /Users/tfarrell/Documents/CODE/SMDB_Companion/src/components/ResultsSkinny.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import Table from "./results/common/Table.svelte";
  import type { FileRecord } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    toggleEnableSelections,
    clearSelected,
    invertSelected,
    toggleSelect,
    toggleChecked,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
    totalChecksStore,
    selectedChecksStore,
    updateCurrentFilter,
    manualFiltersStore,
    filtersStore,
  } from "../stores/results";
  import {
    metadataStore,
    toggleMarkDirty,
    replaceMetadata,
  } from "../stores/metadata";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { databaseStore, setDatabase } from "../stores/database";
  import { isSearching, searchProgressStore } from "../stores/status";
  import { removeRecords, removeSelectedRecords } from "../stores/remove";
  import { isRemove } from "../stores/menu";
  import { algoEnabled } from "../stores/algorithms";

  import {
    CheckSquare,
    Square,
    OctagonX,
    NotebookPenIcon,
    Loader,
    TriangleAlert,
  } from "lucide-svelte";

  export let selectedDb: string | null = null;

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;
  $: selectedItems = $selectedItemsStore;
  $: currentFilter = $currentFilterStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;
  $: filters = $filtersStore;

  let processing = false;
  let loading = true;

  // Handle filter change
  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    updateCurrentFilter(select.value);
  }

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

  let processingBatch = false;

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    // Use setTimeout to allow UI to update before heavy processing
    setTimeout(() => {
      checkSelected();
      processingBatch = false;
    }, 10);
  }
</script>

<div class="block" style="width: 75vw;">
  <div class="header">
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
      <button type="button" class="grid item" on:click={toggleMarkDirty}>
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
        {:else if $databaseStore == null || $databaseStore.name == "" || $databaseStore.name == "Select Database"}
          <button class="cta-button inactive">
            <OctagonX size="18" />
            Remove {totalChecks} Records
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
    <div style="margin-top: 10px; margin-bottom: 20px; padding: 0px;">
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
          <span style="margin-left: 10px">({selectedItems.size} selected)</span>
        {/if}
        {#if processingBatch}
          <div class="batch-processing">
            <Loader size={24} class="spinner" />
            <span>Processing {selectedItems.size} items...</span>
          </div>
        {/if}
      {/if}
    </div>
  {/if}
  <div class="block inner" style="margin-bottom: 15px;">
    {#if $isSearching}
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
    <div class="header" style="margin-bottom: 0px; margin-top: 15px;">
      {#if $isRemove}
        <span>
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

  .select-field {
    flex: 0;
  }

  /* Header styling for headings if needed in the future */
</style>
