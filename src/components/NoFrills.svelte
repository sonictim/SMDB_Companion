<script lang="ts">
  import { Database, Search as SearchIcon, X } from "lucide-svelte";
  import { databaseStore, openDatabase } from "../stores/database";
  import {
    resultsStore,
    selectedItemsStore,
    totalChecksStore,
    selectedChecksStore, // Import the new store
  } from "../stores/results";
  import { showStatus, toggleSearch } from "../stores/status";

  import Table from "./results/Table.svelte";
  import Algorithms from "./search/Algorithms.svelte";
  import Status from "./Status.svelte";
  import Form from "./registration/Form.svelte";
  import RemoveButton from "./results/removeButton.svelte";
  import { isRegistered } from "../stores/registration";

  import { removeRecords, removeSelectedRecords } from "../stores/remove";

  $: results = $resultsStore;
  $: selectedItems = $selectedItemsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore; // Create a reactive reference to selected checks

  let loading = false;

  import { CheckSquare, Square, OctagonX, Loader } from "lucide-svelte";
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
        {#if selectedItems.size > 0}
          <span style="font-size: var(--font-size-md);"
            >({selectedItems.size} selected)</span
          >
        {/if}
      </span>
    </button>
  </div>
  <div class="top-bar-right">
    {#if $databaseStore}
      <button
        class="nav-link"
        on:click={async () => {
          toggleSearch();
        }}
        title="Search for Duplicates"
      >
        <div class="flex items-center gap-2">
          {#if $showStatus}
            <X size={18} />
            <span>Cancel Search</span>
          {:else}
            <SearchIcon size={18} />
            <span>Search for Records</span>
          {/if}
        </div>
      </button>
    {/if}
    {#if !$isRegistered}
      <RemoveButton />
    {:else if $resultsStore.length > 0}
      <button
        class="nav-link"
        on:click={() => {
          if (selectedItems.size > 0) removeSelectedRecords();
          else removeRecords();
        }}
        title="Remove Duplicates"
      >
        <div class="flex items-center gap-2">
          <OctagonX size={18} />
          {#if selectedItems.size > 0}
            Remove {selectedChecks} Selected Records
          {:else}
            <span>Remove {totalChecks} Records</span>
          {/if}
        </div>
      </button>
    {/if}
  </div>
</div>

<div class="block">
  {#if loading}
    <p class="ellipsis">Loading data...</p>
  {:else if $showStatus}
    <Status />
  {:else if $resultsStore.length > 0}
    {#if $isRegistered}
      <Table />
    {:else}
      <div class="header">
        <span style="font-size: var(--font-size-lg)">
          <h2>Search Results:</h2>
          <p>
            {totalChecks} of {results.length} Records marked for Removal
          </p>
          <p>Registration Required to View Full Results</p>
        </span>
      </div>
      <Form />
    {/if}
  {:else}
    <div class="grid" style="margin-top: 40px; margin-left: 40px">
      <Algorithms />
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

  /* font-weight: bold; */
  .block {
    background-color: var(--secondary-bg);
    padding: 0px 0px;
    /* padding-bottom: 20px; */
    border-radius: 8px;
    flex: 1;
    /* Allow features to grow and shrink */
    margin-bottom: 0px;
    /* margin-top: -20px; */
    /* Adjust as needed */
    display: flex;
    flex-direction: column;
    /* Stack items vertically */
    /* Full viewport height */
  }

  .grid {
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, auto);
    grid-auto-flow: column;
  }
</style>
