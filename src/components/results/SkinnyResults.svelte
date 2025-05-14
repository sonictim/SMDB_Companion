<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { message } from "@tauri-apps/plugin-dialog";

  import BaseResults from "./common/BaseResults.svelte";
  import ResultsUI from "./common/ResultsUI.svelte";
  import ResultsActions from "./common/ResultsActions.svelte";

  import type { FileRecord } from "../../stores/types";
  import { preferencesStore } from "../../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    toggleEnableSelections,
    updateCurrentFilter,
    filtersStore,
    manualFiltersStore,
  } from "../../stores/results";
  import { isSearching, searchProgressStore } from "../../stores/status";
  import { metadataStore } from "../../stores/metadata";
  import { databaseStore } from "../../stores/database";

  // Props
  export let isRemove: boolean = false;
  export let selectedDb: string | null = null;

  // Local state
  let processing = false;
  let playingAudio: string | null = null;
  let audioPlayer: HTMLAudioElement;

  // Reactive stores
  $: pref = $preferencesStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: filters = $filtersStore;
  $: currentFilter = $currentFilterStore;
  $: searchProgress = $searchProgressStore;

  // Initialize minimalist view settings
  let searchVisible = true;
  let actionsVisible = false;

  // Event handlers
  function toggleSearchVisibility() {
    searchVisible = !searchVisible;
  }

  function toggleActionsVisibility() {
    actionsVisible = !actionsVisible;
  }

  function handleActionCompleted({
    detail,
  }: CustomEvent<{ action: string; success: boolean }>) {
    const { action, success } = detail;

    if (success) {
      // Handle successful actions
      if (action === "removeFiltered" || action === "removeSelected") {
        // Additional cleanup or UI updates specifically for skinny view
        toggleActionsVisibility();
      }
    } else {
      // Handle failed actions
      console.error(`Action ${action} failed`);
    }
  }

  // Handle playing audio (simplified for skinny view)
  async function playAudio(item: FileRecord) {
    try {
      const audioUrl = await invoke("get_audio_url", {
        path: item.path + "/" + item.filename,
      });

      playingAudio = audioUrl as string;

      if (audioPlayer) {
        audioPlayer.src = playingAudio;
        audioPlayer.play();
      }
    } catch (error) {
      console.error("Error playing audio:", error);
      message("Error playing audio: " + error);
    }
  }

  // Filter handling
  function handleFilterChange(filterName: string) {
    updateCurrentFilter(filterName);
  }

  // Component lifecycle
  onMount(() => {
    audioPlayer = new Audio();

    // Listen for keyboard shortcuts to toggle panels
    const unsubscribeKeydown = listen("keydown", (event: any) => {
      if (event.payload.key === "s" && event.payload.ctrlKey) {
        toggleSearchVisibility();
      } else if (event.payload.key === "a" && event.payload.ctrlKey) {
        toggleActionsVisibility();
      }
    });

    return () => {
      unsubscribeKeydown.then((fn) => fn());
    };
  });

  onDestroy(() => {
    // Clean up audio player
    if (audioPlayer) {
      audioPlayer.pause();
      audioPlayer.src = "";
    }
  });
</script>

<BaseResults
  {isRemove}
  {selectedDb}
  mode="skinny"
  on:actionCompleted={handleActionCompleted}
>
  <div slot="header">
    <div class="skinny-header">
      <div class="skinny-title">
        <button class="toggle-button" on:click={toggleSearchVisibility}>
          {searchVisible ? "▼" : "▶"}
        </button>
        <h2>Results</h2>
        <span class="results-count">{filteredItems.length} items</span>
      </div>

      {#if searchVisible}
        <div class="filter-bar">
          <select
            class="filter-select compact"
            bind:value={$currentFilterStore}
            on:change={(e) => handleFilterChange(e.target.value)}
          >
            {#each Object.keys(filters) as filterName}
              <option value={filterName}>
                {filterName} ({filters[filterName].count})
              </option>
            {/each}
          </select>

          <button
            class="actions-toggle"
            on:click={toggleActionsVisibility}
            title="Toggle actions panel"
          >
            {actionsVisible ? "Hide Actions" : "Show Actions"}
          </button>
        </div>
      {/if}
    </div>
  </div>

  <div slot="results-list" class="skinny-layout">
    {#if actionsVisible}
      <div class="skinny-actions">
        <ResultsActions
          mode="skinny"
          {processing}
          on:action={({ detail }) => {
            // Handle actions from the actions component
            if (detail.name === "exportSelected") {
              // Handle export selected
              console.log("Export selected items");
            } else if (detail.name === "exportAll") {
              // Handle export all
              console.log("Export all items");
            }
          }}
        >
          <div slot="custom-actions-top">
            <button
              class="close-button"
              on:click={toggleActionsVisibility}
              title="Close actions panel"
            >
              ×
            </button>
          </div>
        </ResultsActions>
      </div>
    {/if}

    <div class="skinny-content" class:with-actions={actionsVisible}>
      <ResultsUI
        items={filteredItems}
        {selectedItems}
        mode="skinny"
        on:playAudio={({ detail }) => playAudio(detail.item)}
      />
    </div>
  </div>

  <div slot="footer">
    <div class="skinny-footer">
      {#if $isSearching}
        <div class="search-progress">
          <progress value={searchProgress} max="100"></progress>
          <span>Searching... {searchProgress}%</span>
        </div>
      {:else if processing}
        <div class="processing-indicator">Processing...</div>
      {:else if playingAudio}
        <div class="mini-player">
          <span>▶</span>
          <span class="filename">{playingAudio.split("/").pop()}</span>
        </div>
      {/if}

      <div class="footer-controls">
        <button
          class="toggle-button skinny"
          on:click={toggleEnableSelections}
          title="Toggle selection mode"
        >
          {$enableSelectionsStore ? "Selection Mode" : "View Mode"}
        </button>

        {#if isRemove && selectedItems.size > 0}
          <button
            class="remove-button"
            on:click={() => toggleActionsVisibility()}
            title="Show remove options"
          >
            Remove Selected ({selectedItems.size})
          </button>
        {/if}
      </div>
    </div>
  </div>
</BaseResults>

<style>
  .skinny-header {
    padding: 8px;
    border-bottom: 1px solid var(--inactive-color);
  }

  .skinny-title {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .skinny-title h2 {
    margin: 0;
    font-size: 1.2rem;
  }

  .results-count {
    color: var(--inactive-color);
    font-size: 0.9rem;
  }

  .toggle-button {
    background: none;
    border: none;
    color: var(--accent-color);
    cursor: pointer;
    font-size: 1rem;
    padding: 4px 8px;
  }

  .filter-bar {
    margin-top: 8px;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .filter-select.compact {
    padding: 4px 8px;
    font-size: 0.9rem;
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
    background-color: var(--secondary-bg);
    color: var(--text-color);
    flex: 1;
  }

  .actions-toggle {
    background-color: var(--accent-color);
    color: white;
    border: none;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .skinny-layout {
    display: flex;
    height: 100%;
    position: relative;
  }

  .skinny-actions {
    width: 200px;
    border-right: 1px solid var(--inactive-color);
  }

  .skinny-content {
    flex: 1;
    overflow: hidden;
  }

  .skinny-content.with-actions {
    margin-left: auto;
  }

  .skinny-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px;
    background-color: var(--secondary-bg);
    border-top: 1px solid var(--inactive-color);
    font-size: 0.9rem;
  }

  .search-progress {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-progress progress {
    width: 100px;
  }

  .mini-player {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent-color);
  }

  .filename {
    max-width: 150px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .footer-controls {
    display: flex;
    gap: 8px;
  }

  .toggle-button.skinny {
    background-color: var(--accent-color);
    color: white;
    border-radius: 4px;
    font-size: 0.8rem;
    padding: 4px 8px;
  }

  .remove-button {
    background-color: var(--warning-color);
    color: white;
    border: none;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .close-button {
    background: none;
    border: none;
    color: var(--inactive-color);
    font-size: 1.5rem;
    cursor: pointer;
    padding: 0;
    position: absolute;
    top: 5px;
    right: 5px;
  }

  .close-button:hover {
    color: var(--text-color);
  }

  .processing-indicator {
    color: var(--accent-color);
    font-weight: 500;
  }
</style>
