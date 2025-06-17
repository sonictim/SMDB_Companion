<script lang="ts">
  import {
    Database,
    Search as SearchIcon,
    FilesIcon,
    Settings2,
  } from "lucide-svelte";
  import { databaseStore, openDatabase } from "../stores/database";
  import {
    togglePreferencesWindow,
    viewStore,
    showSearchView,
    showResultsView,
    showSplitView,
    showSearchPopup,
  } from "../stores/menu";

  // Bind the local variables to viewStore
  $: view = $viewStore;

  function toggleSplitView() {
    if (view === "split") {
      showResultsView();
    } else {
      $showSearchPopup = true;
      // showSplitView();
    }
  }
</script>

<div class="top-bar">
  <div class="top-bar-left">
    <button class="nav-link" on:click={() => openDatabase(false)}>
      <Database size={18} />
      <span style="font-size: var(--font-size-xl);">
        {$databaseStore?.name || "Select Database"}
        {#if $databaseStore}
          <span style="font-size: var(--font-size-md);"
            >{$databaseStore.size} total records</span
          >
        {/if}
      </span>
    </button>
  </div>
  <div class="top-bar-right">
    <button
      class="nav-link {view === 'search' || view === 'split' ? 'active' : ''}"
      on:click={(action) => {
        if (action.metaKey) {
          // Toggle results view on/off while keeping search view on
          showSplitView();
        } else {
          // Just show search view
          // $showSearchPopup = true;
          toggleSplitView();
          // showSearchView();
        }
        // view = "search";
        console.log("Search tab clicked", view);
      }}
      title="Hold CMD to toggle search bar"
    >
      <div class="flex items-center gap-2">
        <SearchIcon size={18} />
        <span>Search</span>
      </div>
    </button>
    <!-- <button
      class="nav-link {view === 'results' || view === 'split' ? 'active' : ''}"
      on:click={(action) => {
        if (action.metaKey) {
          // Toggle search view on/off while keeping results view on
          showSplitView();
        } else {
          // Just show results view
          showResultsView();
        }
        view = "results";
        console.log("Results tab clicked", view);
      }}
      title="Hold CMD to toggle split view"
    >
      <div class="flex items-center gap-2">
        <FilesIcon size={18} />
        <span>Results</span>
      </div>
    </button> -->
    <button class="nav-link" on:click={togglePreferencesWindow}>
      <div class="flex items-center gap-2">
        <Settings2 size={18} /> Options
      </div>
    </button>
  </div>
</div>

<style>
  /* Styles for the top bar would go here if they're not in the global CSS */
</style>
