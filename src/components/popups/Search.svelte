<script lang="ts">
  import { onMount } from "svelte";
  import { Search, Database, Folder } from "lucide-svelte";
  import { toggleSearch } from "../../stores/status";
  import { databaseStore, setDatabase } from "../../stores/database";
  import { searchFoldersStore, toggleFolderSearch } from "../../stores/search";
  import Algorithm from "../search/Algorithms.svelte";
  import SearchDatabaseContent from "./SearchDatabaseContent.svelte";
  import SearchFoldersContent from "./SearchFoldersContent.svelte";
  import { Popup } from "../../stores/menu";

  // Tab state - derived from the current popup type
  $: isDbMode = $Popup === "search";

  // State managed by child components
  let selectedDb: { url: string; name: string } | null = null;
  let selectedFolders = new Set<string>();

  onMount(() => {
    selectedDb =
      $databaseStore && $databaseStore.name
        ? { url: $databaseStore.url, name: $databaseStore.name }
        : null;
    console.log("Search component mounted, popup type:", $Popup);
  });

  function switchToDatabase() {
    Popup.set("search");
  }

  function switchToFileSystem() {
    Popup.set("searchFolder");
  }

  async function handleSearch() {
    if (isDbMode) {
      // Only proceed if a database is selected
      if (selectedDb) {
        await setDatabase(selectedDb.url, false);
        toggleSearch();
      }
    } else {
      toggleFolderSearch();
    }
  }

  // Computed property for button disabled state
  $: isSearchDisabled = isDbMode && !selectedDb;
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block">
    <!-- Tab Slider/Switcher -->
    <div class="tab-container">
      <div class="tab-slider" class:database-mode={isDbMode}>
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div class="tab" class:active={isDbMode} on:click={switchToDatabase}>
          <Database size="16" />
          Database Search
        </div>
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div class="tab" class:active={!isDbMode} on:click={switchToFileSystem}>
          <Folder size="16" />
          File System Search
        </div>
        <div class="slider-indicator" class:right={!isDbMode}></div>
      </div>
    </div>

    <div class="header">
      <span class="tooltip-trigger">
        <span class="tooltip-text" style="height: 340%;">
          {isDbMode
            ? "Select a Database to Search."
            : "Add or remove folders to tell the app where to search for files."}
        </span>
        <h2>{isDbMode ? "Database Search" : "File System Search"}</h2>
      </span>
      <span>
        <button
          class="cta-button {(isDbMode && selectedDb) ||
          (!isDbMode && $searchFoldersStore.length > 0)
            ? ''
            : 'inactive'}"
          class:disabled={isSearchDisabled}
          disabled={isSearchDisabled}
          on:click={handleSearch}
        >
          <Search size="16" />
          Search {isDbMode
            ? selectedDb
              ? selectedDb.name
              : ""
            : "Folders in List"}
        </button>
      </span>
    </div>

    <div class="split">
      <span>
        {#if isDbMode}
          <!-- Database Search Content -->
          <SearchDatabaseContent bind:selectedDb />
        {:else}
          <!-- File System Search Content -->
          <SearchFoldersContent bind:selectedFolders />
        {/if}
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

  .tab-container {
    margin-bottom: 20px;
  }

  .tab-slider {
    position: relative;
    display: flex;
    background-color: var(--secondary-bg, rgba(255, 255, 255, 0.1));
    border-radius: 12px;
    padding: 4px;
    gap: 2px;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 12px 16px;
    border-radius: 8px;
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    z-index: 2;
    color: var(--inactive-color);
  }

  .tab.active {
    color: var(--primary-bg);
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .tab:hover {
    color: var(--text-color);
  }

  .slider-indicator {
    position: absolute;
    top: 4px;
    left: 4px;
    width: calc(50% - 3px);
    height: calc(100% - 8px);
    background: linear-gradient(
      135deg,
      var(--hover-color),
      var(--accent-color)
    );
    border-radius: 8px;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow:
      0 2px 8px var(--hover-color, rgba(0, 122, 204, 0.3)),
      0 1px 3px rgba(0, 0, 0, 0.2);
    z-index: 1;
  }

  .slider-indicator.right {
    transform: translateX(calc(100% + 2px));
  }

  .split {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr;
    gap: 10px;
    height: 100%;
  }

  .algo {
    display: grid;
    grid-template-rows: 9;
    gap: 10px;
    height: 100%;
  }

  /* Enhanced visual effects for the slider */
  .tab-slider.database-mode {
    background: linear-gradient(
      90deg,
      var(--primary-color-alpha, rgba(0, 122, 204, 0.15)) 0%,
      rgba(255, 255, 255, 0.1) 50%,
      rgba(255, 255, 255, 0.1) 100%
    );
  }

  .tab-slider:not(.database-mode) {
    background: linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.1) 0%,
      rgba(255, 255, 255, 0.1) 50%,
      var(--primary-color-alpha, rgba(0, 122, 204, 0.15)) 100%
    );
  }

  /* Add a subtle glow effect when hovering */
  .tab-slider:hover .slider-indicator {
    box-shadow:
      0 4px 12px var(--primary-color-alpha, rgba(0, 122, 204, 0.4)),
      0 2px 6px rgba(0, 0, 0, 0.3);
  }
</style>
