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
  import SearchButton from "./search/SearchButton.svelte";
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
</script>

<div class="page-columns">
  <div class="block">
    <div class="header">
      <h2>Search Algorithms</h2>
      <SearchButton />
    </div>

    <div class="grid">
      <Algorithms />
    </div>
    {#if $showStatus}
      <Status />
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

    gap: 0px;
  }

  :global(.checkbox.checked) {
    color: var(--accent-color);
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
  }
</style>
