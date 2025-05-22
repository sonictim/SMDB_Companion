<script lang="ts">
  import { X, Search, SearchCheck } from "lucide-svelte";
  import { onMount } from "svelte";
  import { databaseStore } from "../stores/database";
  $: database = $databaseStore;

  import { preferencesStore } from "../stores/preferences";
  import {
    showStatus,
    initializeSearchListeners,
    toggleSearch, // Import the moved functions
  } from "../stores/status";
  import Algorithms from "./search/Algorithms.svelte";
  import Status from "./Status.svelte";
  import Metadata from "./Metadata.svelte";

  onMount(() => {
    initializeSearchListeners().then(() => {
      console.log("Search component mounted, showStatus:", $showStatus);
    });

    const unsubscribe = showStatus.subscribe((value) => {
      console.log("showStatus changed:", value);
    });

    return () => {
      unsubscribe();
    };
  });

  function checkAnyAlgorithmEnabled() {
    return $preferencesStore.algorithms.some((algo) => algo.enabled);
  }
</script>

<div class="page-columns">
  <div class="block">
    <div class="header">
      <h2>Search Algorithms</h2>
      {#if database == null || database.name == "" || database.name == "Select Database" || !checkAnyAlgorithmEnabled()}
        <button class="cta-button inactive">
          <SearchCheck size={18} />
          <span>Search for Records</span>
        </button>
      {:else}
        <button
          class="cta-button {$showStatus ? 'cancel' : ''}"
          on:click={async () => {
            let result = await toggleSearch();
          }}
        >
          <div class="flex items-center gap-2">
            {#if $showStatus}
              <X size={18} />
              <span>Cancel</span>
            {:else}
              <SearchCheck size={18} />
              <span>Search for Records</span>
            {/if}
          </div>
        </button>
      {/if}
    </div>
    {#if $showStatus}
      <Status />
    {:else}
      <div class="grid">
        <Algorithms />
      </div>
      <span style="margin-left: 255px"> </span>
    {/if}
  </div>

  <Metadata />
</div>

<style>
  .grid {
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, auto);
    grid-auto-flow: column;
  }

  .page-columns {
    display: grid;
    grid-template-columns: repeat(1, 1fr); /* 3 equal columns */

    gap: 10px;
  }

  :global(.checkbox.checked) {
    color: var(--accent-color);
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
  }
</style>
