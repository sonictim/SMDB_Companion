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
  } from "../stores/menu";

  // Bind the local variables to viewStore
  $: view = $viewStore;
</script>

<div class="top-bar">
  <div class="top-bar-left">
    <button class="nav-link" on:click={() => openDatabase(false)}>
      <Database size={18} />
      <span style="font-size: 24px;">
        {$databaseStore?.name || "Select Database"}
        {#if $databaseStore}
          <span style="font-size: 14px;"
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
          showSearchView();
        }
        view = "search";
        console.log("Search tab clicked", view);
      }}
      title="Hold CMD to toggle split view"
    >
      <div class="flex items-center gap-2">
        <SearchIcon size={18} />
        <span>Search</span>
      </div>
    </button>
    <button
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
    </button>
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
