<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { onMount, onDestroy } from "svelte";
  import { Square, CheckSquare, OctagonX, Folder, Search } from "lucide-svelte";
  // Import from main store instead
  import { preferencesStore } from "../../stores/preferences";

  import { toggleSearch } from "../../stores/status";
  import {
    recentDbStore,
    openDatabase,
    databaseStore,
    setDatabase,
  } from "../../stores/database";
  import { get } from "svelte/store";
  import Algorithm from "../search/Algorithms.svelte";
  import { SearchFolderPopup } from "../../stores/menu";

  let selectedDb: { url: string; name: string } | null =
    $databaseStore && $databaseStore.name
      ? { url: $databaseStore.url, name: $databaseStore.name }
      : null;

  onMount(() => {
    // Initialize or load any necessary data here
    selectedDb =
      $databaseStore && $databaseStore.name
        ? { url: $databaseStore.url, name: $databaseStore.name }
        : null;
    console.log("SearchDatabase component mounted");
  });

  function toggleDb(item: { url: string; name: string }) {
    selectedDb = item;
    // setDatabase(item.url, false);
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block">
    <div class="header">
      <span class="tooltip-trigger">
        <span class="tooltip-text" style="height: 340%;">
          Select a Database to Search.
        </span>
        <h2>Database Search</h2>
      </span>
      <span>
        <button class="cta-button" on:click={() => toggleSearch()}>
          <Search size="16" />
          Search
        </button>
      </span>
    </div>

    <div class="split">
      <span>
        <div class="block inner" style="height: 90%; overflow: hidden;">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <VirtualList items={Array.from($recentDbStore)} let:item>
            <div
              on:click={() => toggleDb(item)}
              class="list-item"
              class:selected-item={selectedDb && selectedDb.name === item.name}
              class:unselected-item={!selectedDb ||
                selectedDb.name !== item.name}
            >
              <!-- Display the name but track the full object -->
              {item.name}
              <!-- <span class="url-subtitle">{item.url}</span> -->
            </div>
          </VirtualList>
        </div>
        <div class="header" style="margin-top: 15px;">
          <div class="button-group">
            <button
              class="cta-button small"
              on:click={async () => {
                let url = await openDatabase(false);
                if (!url) return;
                let name = url.split("/").pop();
                name = name ? name.replace(".sqlite", "") : "New Database";
                selectedDb = {
                  name,
                  url,
                };
              }}
            >
              <Folder size="12" />
              Add Database</button
            >
            <!-- <button
              class="cta-button cancel small"
              on:click={() => removeSearchFolders([...selectedFolders])}
            >
              <OctagonX size="12" />
              Remove Selected
            </button> -->
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
  .url-subtitle {
    font-size: 0.8em;
  }
</style>
