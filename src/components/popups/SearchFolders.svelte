<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { Square, CheckSquare, OctagonX, Folder, Search } from "lucide-svelte";
  // Import from main store instead
  import { preferencesStore } from "../../stores/preferences";

  import {
    searchFoldersStore,
    addSearchFolders,
    removeSearchFolders,
    clearSearchFolders,
    toggleFolderSearch,
  } from "../../stores/search";
  import { get } from "svelte/store";
  import Algorithm from "../search/Algorithms.svelte";

  let selectedFolders = new Set<string>();

  function toggleFolder(item: string) {
    if (selectedFolders.has(item)) {
      selectedFolders.delete(item);
    } else {
      selectedFolders.add(item);
    }
    selectedFolders = new Set(selectedFolders); // Ensure reactivity
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block">
    <div class="header">
      <span class="tooltip-trigger">
        <span class="tooltip-text" style="height: 340%;">
          Add or remove folders to tell the app where to search for files.
        </span>
        <h2>File System Search</h2>
      </span>
      <span>
        <button class="cta-button" on:click={() => toggleFolderSearch()}>
          <Search size="16" />
          Search
        </button>
      </span>
    </div>

    <div class="split">
      <span>
        <div class="block inner" style="height: 90%; overflow: hidden;">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <VirtualList items={Array.from($searchFoldersStore)} let:item>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              on:click={() => toggleFolder(item)}
              class="list-item"
              class:selected-item={selectedFolders.has(item)}
              class:unselected-item={!selectedFolders.has(item)}
            >
              {item}
            </div>
          </VirtualList>
        </div>
        <div class="header" style="margin-top: 15px;">
          <div class="button-group">
            <button class="cta-button small" on:click={addSearchFolders}>
              <Folder size="12" />
              Add Folders to Search</button
            >
            <button
              class="cta-button cancel small"
              on:click={() => removeSearchFolders([...selectedFolders])}
            >
              <OctagonX size="12" />
              Remove Selected
            </button>
          </div>
        </div>
      </span>
      <div class="algo">
        <Algorithm />
      </div>
    </div>
  </div>
</div>

<style>
  .popup {
    background-color: var(--primary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 8px;
    padding: 24px;
    max-width: 800px;
    width: 90vw;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    color: var(--text-color);
  }
  .split {
    display: grid;
    grid-template-columns: 1fr 1fr; /* Two equal columns */
    grid-template-rows: 1fr; /* Single row */
    gap: 10px;
    height: 100%;
  }
  .algo {
    display: grid;
    grid-template-rows: 9; /* Two equal columns */
    gap: 10px;
    height: 100%;
  }
</style>
