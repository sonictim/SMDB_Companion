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
  import Matches from "./prefs/Match.svelte";
  import MatchPage from "./prefs/MatchPage.svelte";
  import SafeFolders from "./prefs/SafeFolders.svelte";
  import Order from "./prefs/Order.svelte";
  import WaveFormSearch from "./prefs/WaveFormSearch.svelte";
  import Tags from "./prefs/Tags.svelte";

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
  <div class="preferences-grid">
    <Matches />
    <Order />

    <WaveFormSearch />
    <SafeFolders />
  </div>

  <!-- <Metadata /> -->
</div>

<style>
  .grid {
    grid-template-columns: repeat(2, 1fr);
    grid-template-rows: repeat(3, auto);
    grid-auto-flow: column;
  }

  .page-columns {
    display: grid;
    grid-template-columns: repeat(1, 1fr);
    grid-template-rows: auto 1fr; /* First row auto-sizes, second row takes remaining space */
    gap: 10px;
    height: 100vh;
    overflow: hidden; /* Prevent overflow */
    margin: 0px;
    padding: 0px;
  }

  .preferences-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    grid-template-rows: auto 1fr; /* First row auto-sizes, second row takes remaining space */
    row-gap: 10px; /* Same gap as columns */
    column-gap: 10px; /* Keep normal gap between columns */
    overflow: hidden; /* Prevent the grid from overflowing */
    height: 94%; /* Take full available height */
  }

  :global(.checkbox.checked) {
    color: var(--accent-color);
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
  }
</style>
