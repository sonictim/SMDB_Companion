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
    manualFiltersStore,
    filtersStore,
    toggleSelect,
    clearSelected,
    invertSelected,
  } from "../../stores/results";
  import { isSearching } from "../../stores/status";
  import { metadataStore } from "../../stores/metadata";

  // Props
  export let isRemove: boolean = false;
  export let selectedDb: string | null = null;

  // Local state
  let processing = false;
  let currentView = "grid"; // or 'list'
  let playingAudio: string | null = null;
  let audioPlayer: HTMLAudioElement;

  // Reactive stores
  $: pref = $preferencesStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: filters = $filtersStore;
  $: currentFilter = $currentFilterStore;

  // Event handlers
  function handleActionCompleted({
    detail,
  }: CustomEvent<{ action: string; success: boolean }>) {
    const { action, success } = detail;

    if (success) {
      // Handle successful actions
      if (action === "removeFiltered" || action === "removeSelected") {
        // Additional cleanup or UI updates if needed
      }
    } else {
      // Handle failed actions
      console.error(`Action ${action} failed`);
    }
  }

  // Handle playing audio
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

    // Additional setup and listeners here
  });

  onDestroy(() => {
    // Clean up audio player and listeners
    if (audioPlayer) {
      audioPlayer.pause();
      audioPlayer.src = "";
    }
  });
</script>

<BaseResults
  {isRemove}
  {selectedDb}
  mode="standard"
  on:actionCompleted={handleActionCompleted}
>
  <div slot="header">
    <div class="results-header">
      <div class="results-title">
        <h2>Standard Results View</h2>
        <span class="results-count">{filteredItems.length} items</span>
      </div>

      <div class="filter-controls">
        <select
          class="filter-select"
          bind:value={$currentFilterStore}
          on:change={(e) => handleFilterChange(e.target.value)}
        >
          {#each Object.keys(filters) as filterName}
            <option value={filterName}>
              {filterName} ({filters[filterName].count})
            </option>
          {/each}
        </select>

        <div class="view-toggles">
          <button
            class="view-button"
            class:active={currentView === "grid"}
            on:click={() => (currentView = "grid")}
          >
            Grid
          </button>
          <button
            class="view-button"
            class:active={currentView === "list"}
            on:click={() => (currentView = "list")}
          >
            List
          </button>
        </div>
      </div>
    </div>
  </div>

  <div slot="results-list">
    <div class="split-view">
      <div class="main-content">
        {#if currentView === "grid"}
          <div class="grid-view">
            <ResultsUI
              items={filteredItems}
              {selectedItems}
              mode="standard"
              on:itemClick={({ detail }) => toggleSelect(detail.item.id)}
              on:playAudio={({ detail }) => playAudio(detail.item)}
            />
          </div>
        {:else}
          <div class="list-view">
            <ResultsUI
              items={filteredItems}
              {selectedItems}
              mode="standard"
              isVirtual={false}
              on:itemClick={({ detail }) => toggleSelect(detail.item.id)}
              on:playAudio={({ detail }) => playAudio(detail.item)}
            />
          </div>
        {/if}
      </div>

      <div class="sidebar">
        <ResultsActions
          mode="standard"
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
        />

        {#if filteredItems.length > 0 && selectedItems.size > 0}
          <div class="selection-details">
            <h3>Selection</h3>
            <p>{selectedItems.size} of {filteredItems.length} selected</p>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <div slot="footer">
    <div class="standard-footer">
      <div class="playing-now">
        {#if playingAudio}
          <div class="audio-player">
            <span class="now-playing">Now playing:</span>
            <span class="audio-name">{playingAudio.split("/").pop()}</span>
          </div>
        {:else}
          <span class="no-audio">No audio playing</span>
        {/if}
      </div>

      <div class="status">
        {#if $isSearching}
          <div class="searching-indicator">Searching...</div>
        {:else if processing}
          <div class="processing-indicator">Processing...</div>
        {/if}
      </div>
    </div>
  </div>
</BaseResults>

<style>
  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px;
    border-bottom: 1px solid var(--inactive-color);
  }

  .results-title {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }

  .results-title h2 {
    margin: 0;
  }

  .results-count {
    color: var(--inactive-color);
  }

  .filter-controls {
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .filter-select {
    padding: 6px 12px;
    border-radius: 4px;
    border: 1px solid var(--inactive-color);
    background-color: var(--secondary-bg);
    color: var(--text-color);
  }

  .view-toggles {
    display: flex;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid var(--inactive-color);
  }

  .view-button {
    padding: 6px 12px;
    background: none;
    border: none;
    color: var(--text-color);
    cursor: pointer;
  }

  .view-button.active {
    background-color: var(--accent-color);
    color: white;
  }

  .split-view {
    display: flex;
    height: 100%;
    width: 100%;
  }

  .main-content {
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    width: 260px;
    border-left: 1px solid var(--inactive-color);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
  }

  .selection-details {
    background-color: var(--secondary-bg);
    padding: 12px;
    border-radius: 4px;
  }

  .selection-details h3 {
    margin: 0 0 8px 0;
    font-size: 1rem;
  }

  .standard-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background-color: var(--secondary-bg);
    border-top: 1px solid var(--inactive-color);
  }

  .playing-now {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .now-playing {
    font-weight: 500;
  }

  .no-audio {
    color: var(--inactive-color);
    font-style: italic;
  }

  .status {
    display: flex;
    align-items: center;
  }

  .searching-indicator,
  .processing-indicator {
    color: var(--accent-color);
    font-weight: 500;
  }

  /* Grid view specific styles */
  .grid-view {
    height: 100%;
    width: 100%;
  }

  /* List view specific styles */
  .list-view {
    height: 100%;
    width: 100%;
  }
</style>
