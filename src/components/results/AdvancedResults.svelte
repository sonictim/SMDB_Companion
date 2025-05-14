<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { message } from "@tauri-apps/plugin-dialog";
  import { Database, Search as SearchIcon, X } from "lucide-svelte";
  import { basename, extname } from "@tauri-apps/api/path";

  import BaseResults from "./common/BaseResults.svelte";
  import ResultsUI from "./common/ResultsUI.svelte";
  import ResultsActions from "./common/ResultsActions.svelte";

  import type { FileRecord } from "../../stores/types";
  import { preferencesStore } from "../../stores/preferences";
  import {
    toggleAlgorithm,
    getAlgorithmTooltip,
  } from "../../stores/algorithms";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    enableSelectionsStore,
    toggleSelect,
    toggleChecked,
    totalChecksStore,
    selectedChecksStore,
    clearResults,
  } from "../../stores/results";
  import {
    searchProgressStore,
    isSearching,
    initializeSearchListeners,
    toggleSearch,
  } from "../../stores/status";
  import { metadataStore } from "../../stores/metadata";
  import {
    databaseStore,
    openDatabase,
    getCompareDb,
    setDatabase,
  } from "../../stores/database";
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import {
    removeFilteredRecords,
    removeSelectedRecords,
  } from "../../stores/remove";

  // Props
  export let selectedDb: string | null = null;

  // Local state
  let processing = false;
  let loading = true;
  let searchOptions = {
    advancedMode: true,
    waveformSearch: false,
    exactMatch: true,
    showMatch: true,
    searchTerm: "",
  };
  let algorithms: {
    id: string;
    name: string;
    enabled: boolean;
    description: string;
  }[] = [];
  let compareDbPath: string | null = null;
  let playingAudio: string | null = null;
  let audioPlayer: HTMLAudioElement;

  // Element references
  let resultsContainer: HTMLElement;

  // Reactive stores
  $: pref = $preferencesStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;
  $: searchProgress = $searchProgressStore;
  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  // Initialize algorithms from preferences
  $: {
    if (pref && pref.algorithms) {
      algorithms = [...pref.algorithms];
    }
  }

  // Event handlers
  function handleActionCompleted({
    detail,
  }: CustomEvent<{ action: string; success: boolean }>) {
    const { action, success } = detail;

    if (success) {
      console.log(`Action ${action} completed successfully`);
    } else {
      console.error(`Action ${action} failed`);
    }
  }

  // Algorithm toggle
  function handleAlgorithmToggle(id: string) {
    toggleAlgorithm(id);
    // Re-fetch algorithms after toggle
    algorithms = [...$preferencesStore.algorithms];
  }

  // Search functions
  async function startSearch() {
    if ($isSearching) {
      toggleSearch();
      return;
    }

    if (!selectedDb) {
      // No database selected, open selector
      const dbPath = await openDatabase();
      if (!dbPath) return; // User cancelled
      selectedDb = dbPath;
    }

    if (searchOptions.showMatch && !compareDbPath) {
      // Need to select a compare database
      const dbPath = await getCompareDb();
      if (!dbPath) return; // User cancelled
      compareDbPath = dbPath;
    }

    processing = true;
    try {
      await invoke("start_search", {
        dbPath: selectedDb,
        compareDb: compareDbPath,
        showMatch: searchOptions.showMatch,
        searchTerm: searchOptions.searchTerm,
        waveformSearch: searchOptions.waveformSearch,
        exactMatch: searchOptions.exactMatch,
      });

      toggleSearch();
    } catch (error) {
      console.error("Error starting search:", error);
      message(`Error starting search: ${error}`);
    } finally {
      processing = false;
    }
  }

  // Audio playback
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

  // Component lifecycle
  onMount(async () => {
    loading = true;

    // Initialize audio player
    audioPlayer = new Audio();

    // Set up search listeners
    const unsubscribe = await initializeSearchListeners();

    // Initialize database if provided
    if (selectedDb) {
      try {
        await setDatabase(selectedDb);
      } catch (error) {
        console.error("Error initializing database:", error);
      }
    }

    loading = false;

    return () => {
      if (unsubscribe) unsubscribe();
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

<div class="advanced-container">
  <div class="sidebar">
    <div class="search-panel">
      <h3>Advanced Search</h3>

      <div class="search-options">
        <div class="search-field">
          <label for="search-term">Search Term:</label>
          <input
            type="text"
            id="search-term"
            bind:value={searchOptions.searchTerm}
            placeholder="Enter search term..."
          />
        </div>

        <div class="option-group">
          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={searchOptions.waveformSearch}
            />
            <span>Use Waveform Search</span>
          </label>

          <label class="checkbox-label">
            <input
              type="checkbox"
              bind:checked={searchOptions.exactMatch}
              disabled={!searchOptions.waveformSearch}
            />
            <span>Exact Match</span>
          </label>

          <label class="checkbox-label">
            <input type="checkbox" bind:checked={searchOptions.showMatch} />
            <span>Compare Database</span>
          </label>
        </div>

        {#if searchOptions.showMatch}
          <div class="compare-db">
            <label>Compare DB:</label>
            <div class="db-selector">
              <input
                type="text"
                readonly
                value={compareDbPath
                  ? basename(compareDbPath)
                  : "No DB selected"}
                placeholder="Select a database..."
              />
              <button
                class="icon-button"
                on:click={async () => {
                  const dbPath = await getCompareDb();
                  if (dbPath) compareDbPath = dbPath;
                }}
              >
                <Database size={16} />
              </button>
            </div>
          </div>
        {/if}
      </div>

      <div class="algorithm-section">
        <h4>Algorithms</h4>
        <div class="algorithm-list">
          {#each algorithms as algorithm (algorithm.id)}
            <div class="algorithm-item">
              <label class="checkbox-label tooltip-container">
                <input
                  type="checkbox"
                  checked={algorithm.enabled}
                  on:change={() => handleAlgorithmToggle(algorithm.id)}
                />
                <span>{algorithm.name}</span>
                <span class="tooltip-text"
                  >{getAlgorithmTooltip(algorithm.id)}</span
                >
              </label>
            </div>
          {/each}
        </div>
      </div>

      <div class="search-controls">
        <button
          class="search-button"
          class:searching={$isSearching}
          disabled={processing}
          on:click={startSearch}
        >
          {$isSearching ? "Stop" : "Search"}
          <SearchIcon size={16} />
        </button>

        <button
          class="clear-button"
          disabled={processing || filteredItems.length === 0}
          on:click={() => clearResults()}
        >
          Clear Results
        </button>
      </div>
    </div>

    <div class="actions-panel">
      <ResultsActions mode="advanced" {processing} />
    </div>
  </div>

  <BaseResults
    isRemove={false}
    {selectedDb}
    mode="advanced"
    on:actionCompleted={handleActionCompleted}
  >
    <div slot="header">
      <div class="results-header">
        <h3>
          Results {filteredItems.length > 0 ? `(${filteredItems.length})` : ""}
        </h3>

        {#if $isSearching}
          <div class="search-progress">
            <progress value={$searchProgressStore} max="100"></progress>
            <span>{$searchProgressStore}%</span>
          </div>
        {/if}
      </div>
    </div>

    <div
      slot="results-list"
      class="results-wrapper"
      bind:this={resultsContainer}
    >
      <ResultsUI
        items={filteredItems}
        {selectedItems}
        mode="advanced"
        on:playAudio={({ detail }) => playAudio(detail.item)}
      />
    </div>

    <div slot="footer">
      <div class="advanced-footer">
        <div class="status-info">
          {#if playingAudio}
            <div class="now-playing">
              <span class="play-icon">▶</span>
              <span class="audio-file">{playingAudio.split("/").pop()}</span>
            </div>
          {/if}
        </div>

        <div class="selection-info">
          {#if selectedItems.size > 0}
            <span class="selected-count">
              {selectedItems.size} selected
            </span>
          {/if}

          {#if isBasicEnabled}
            <span class="basic-indicator">Basic mode active</span>
          {/if}
        </div>
      </div>
    </div>
  </BaseResults>
</div>

<style>
  .advanced-container {
    display: flex;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .sidebar {
    width: 280px;
    border-right: 1px solid var(--inactive-color);
    display: flex;
    flex-direction: column;
    background-color: var(--secondary-bg);
    overflow-y: auto;
  }

  .search-panel {
    padding: 16px;
    border-bottom: 1px solid var(--inactive-color);
  }

  .search-panel h3 {
    margin-top: 0;
    margin-bottom: 16px;
    color: var(--accent-color);
  }

  .search-options {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .search-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .search-field input {
    padding: 8px;
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
    background-color: var(--primary-bg);
    color: var(--text-color);
  }

  .option-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }

  .compare-db {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .db-selector {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .db-selector input {
    flex: 1;
    padding: 6px;
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
    background-color: var(--primary-bg);
    color: var(--text-color);
    font-size: 0.9rem;
  }

  .icon-button {
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--accent-color);
    color: white;
    border: none;
    border-radius: 4px;
    width: 28px;
    height: 28px;
    cursor: pointer;
  }

  .algorithm-section {
    margin-top: 16px;
  }

  .algorithm-section h4 {
    margin-top: 0;
    margin-bottom: 8px;
  }

  .algorithm-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background-color: var(--primary-bg);
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
  }

  .algorithm-item {
    position: relative;
  }

  .tooltip-container {
    position: relative;
    display: inline-block;
  }

  .tooltip-text {
    visibility: hidden;
    width: 250px;
    background-color: var(--secondary-bg);
    color: var(--text-color);
    text-align: left;
    padding: 8px;
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
    font-size: 0.8rem;

    /* Position tooltip */
    position: absolute;
    z-index: 1;
    left: 100%;
    top: 0;
    margin-left: 8px;

    opacity: 0;
    transition: opacity 0.3s;
  }

  .tooltip-container:hover .tooltip-text {
    visibility: visible;
    opacity: 1;
  }

  .search-controls {
    margin-top: 16px;
    display: flex;
    gap: 8px;
  }

  .search-button,
  .clear-button {
    padding: 8px 16px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    font-weight: 500;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-button {
    background-color: var(--accent-color);
    color: white;
    flex: 1;
  }

  .search-button:hover:not(:disabled) {
    background-color: var(--hover-color);
  }

  .search-button.searching {
    background-color: var(--warning-color);
  }

  .clear-button {
    background-color: var(--inactive-color);
    color: white;
  }

  .actions-panel {
    padding: 16px;
    overflow-y: auto;
    flex: 1;
  }

  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--inactive-color);
  }

  .results-header h3 {
    margin: 0;
  }

  .search-progress {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-progress progress {
    width: 150px;
  }

  .results-wrapper {
    height: 100%;
    width: 100%;
  }

  .advanced-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    background-color: var(--secondary-bg);
    border-top: 1px solid var(--inactive-color);
  }

  .now-playing {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent-color);
  }

  .play-icon {
    font-size: 1.2rem;
  }

  .audio-file {
    max-width: 250px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selection-info {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .selected-count {
    background-color: var(--accent-color);
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
  }

  .basic-indicator {
    background-color: var(--warning-color);
    color: white;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
  }
</style>
