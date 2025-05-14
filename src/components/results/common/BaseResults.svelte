<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import type { FileRecord } from "../../../stores/types";
  import { preferencesStore } from "../../../stores/preferences";
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
    filtersStore,
  } from "../../../stores/results";
  import { metadataStore } from "../../../stores/metadata";
  import { databaseStore, setDatabase } from "../../../stores/database";
  import { isSearching } from "../../../stores/status";
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import {
    removeFilteredRecords,
    removeSelectedRecords,
  } from "../../../stores/remove";

  // Props
  export let isRemove: boolean = false;
  export let selectedDb: string | null = null;
  export let mode: "standard" | "skinny" | "advanced" = "standard";

  // Event dispatcher for component events
  const dispatch = createEventDispatcher<{
    itemSelected: { item: FileRecord };
    actionCompleted: { action: string; success: boolean };
  }>();

  // Reactive stores
  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: currentFilter = $currentFilterStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;

  // Common state
  let processing = false;
  let loading = true;
  let parentElement: HTMLElement;
  let virtualizer: any; // Type should be ReturnType<typeof createVirtualizer>

  // Initialize virtualizer
  onMount(() => {
    loading = true;

    // Initialize any listeners or setup code here

    loading = false;
  });

  onDestroy(() => {
    // Clean up any listeners or resources here
  });

  // Handle item click
  function handleItemClick(item: FileRecord) {
    toggleSelect(item.id);
    dispatch("itemSelected", { item });
  }

  // Remove functionality
  async function handleRemoveFiltered() {
    if (processing) return;

    processing = true;
    const success = await removeFilteredRecords();
    processing = false;

    dispatch("actionCompleted", {
      action: "removeFiltered",
      success,
    });
  }

  async function handleRemoveSelected() {
    if (processing) return;

    processing = true;
    const success = await removeSelectedRecords();
    processing = false;

    dispatch("actionCompleted", {
      action: "removeSelected",
      success,
    });
  }
</script>

<div class="base-results-container">
  <slot name="header">
    <!-- Default header content if not provided -->
    <div class="default-header">
      <h2>Results</h2>
    </div>
  </slot>

  <div class="results-content" class:processing class:loading>
    <slot name="filters">
      <!-- Default filter content if not provided -->
    </slot>

    <slot name="results-list">
      <!-- Default results list if not provided -->
      <div class="default-results-list">
        {#if loading}
          <div class="loading-indicator">Loading results...</div>
        {:else if filteredItems.length === 0}
          <div class="empty-results">No results found</div>
        {:else}
          <div class="results-count">{filteredItems.length} items</div>
        {/if}
      </div>
    </slot>
  </div>

  <slot name="footer">
    <!-- Default footer content if not provided -->
    <div class="default-footer">
      {#if isRemove}
        <div class="action-buttons">
          <button
            class="action-button remove-all"
            disabled={processing || filteredItems.length === 0}
            on:click={handleRemoveFiltered}
          >
            Remove All
          </button>
          <button
            class="action-button remove-selected"
            disabled={processing || selectedItems.size === 0}
            on:click={handleRemoveSelected}
          >
            Remove Selected
          </button>
        </div>
      {/if}
    </div>
  </slot>
</div>

<style>
  .base-results-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
  }

  .results-content {
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .results-content.processing::after {
    content: "";
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }

  .loading-indicator,
  .empty-results {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    width: 100%;
    color: var(--inactive-color);
  }

  .default-footer {
    display: flex;
    justify-content: flex-end;
    padding: 10px;
    background-color: var(--secondary-bg);
    border-top: 1px solid var(--inactive-color);
  }

  .action-buttons {
    display: flex;
    gap: 10px;
  }

  .action-button {
    padding: 8px 16px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-weight: 500;
  }

  .action-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .remove-all {
    background-color: var(--warning-color);
    color: white;
  }

  .remove-all:hover:not(:disabled) {
    background-color: var(--warning-hover);
  }

  .remove-selected {
    background-color: var(--accent-color);
    color: white;
  }

  .remove-selected:hover:not(:disabled) {
    background-color: var(--hover-color);
  }
</style>
