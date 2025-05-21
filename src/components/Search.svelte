<script lang="ts">
  import {
    X,
    Search,
    SearchCheck,
    SearchCode,
    AlertCircle,
    Loader,
    Square,
    CheckSquare,
    NotebookPenIcon,
  } from "lucide-svelte";
  import { onMount, onDestroy } from "svelte";
  import { databaseStore, openSqliteFile } from "../stores/database";
  $: database = $databaseStore;

  let isFinding = false;

  import {
    getAlgorithmTooltip,
    getAlgoClass,
    toggleAlgorithm,
  } from "../stores/algorithms";
  import { preferencesStore } from "../stores/preferences";
  import { metadataStore, findMetadata } from "../stores/metadata";
  import {
    showStatus,
    searchProgressStore,
    initializeSearchListeners,
    toggleSearch, // Import the moved functions
  } from "../stores/status";
  import { getFilenameWithoutExtension } from "../stores/utils";
  import Algorithms from "./search/Algorithms.svelte";
  import Status from "./Status.svelte";

  async function getCompareDb() {
    try {
      let compareDb = await openSqliteFile();
      if (compareDb) {
        preferencesStore.update((prefs) => ({
          ...prefs,
          algorithms: prefs.algorithms.map((algo) => {
            if (algo.id === "dbcompare") {
              console.log("Updating dbcompare:", algo, "New DB:", compareDb);
              return { ...algo, enabled: true, db: compareDb };
            }
            return algo;
          }),
        }));
      }
    } catch (error) {
      console.error("Error selecting file:", error);
    }
  }

  $: metadata = metadataStore;
  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  function toggleCaseSensitivity() {
    metadataStore.update((meta) => ({
      ...meta,
      case_sensitive: !meta.case_sensitive,
    }));
  }

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

  function searchForMetadata() {
    isFinding = true;
    findMetadata();
    isFinding = false;
  }
</script>

<div class="page-columns">
  <div class="block" style="height: 40vh">
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
    <!-- </div> -->
  </div>

  <div class="block" style="height: 100%; margin-top: 20px">
    <div class="header">
      <h2>Metadata Replacement</h2>
      {#if database == null || database.name == "" || database.name == "Select Database" || $metadata.find == "" || $metadata.find == null}
        <button class="cta-button inactive" style="width: 225px">
          <SearchCode size={18} />
          <span> Find Metadata </span>
        </button>
      {:else}
        <button
          class="cta-button"
          style="width: 125px"
          on:click={searchForMetadata}
        >
          <SearchCode size={18} />
          <span> Find Metadata </span>
        </button>
      {/if}
    </div>
    {#if isFinding}
      <div class="block inner">
        <span>
          <Loader
            size={24}
            class="spinner ml-2"
            style="color: var(--accent-color)"
          />
          {$searchProgressStore.searchMessage}
        </span>
        <div class="progress-container">
          <div
            class="progress-bar"
            style="width: {$searchProgressStore.searchProgress}%"
          ></div>
        </div>
        <span>
          {$searchProgressStore.subsearchMessage}
        </span>
        <div class="progress-container">
          <div
            class="progress-bar"
            style="width: {$searchProgressStore.subsearchProgress}%"
          ></div>
        </div>
      </div>
    {:else}
      <div class="input-group2">
        <label for="case-sensitive">
          <button
            type="button"
            class="grid item"
            on:click={toggleCaseSensitivity}
          >
            {#if $metadata.case_sensitive}
              <CheckSquare size={20} class="checkbox checked" />
            {:else}
              <Square size={20} class="checkbox" />
            {/if}
            <span>Case Sensitive</span>
          </button>
        </label>
      </div>

      <div class="input-group">
        <label for="find-text">Find:</label>
        <input
          type="text"
          id="find-text"
          bind:value={$metadata.find}
          placeholder="Enter text to find"
          class="input-field"
        />
      </div>

      <div class="input-group">
        <label for="replace-text">Replace:</label>
        <input
          type="text"
          id="replace-text"
          bind:value={$metadata.replace}
          placeholder="Enter text to replace"
          class="input-field"
        />
      </div>

      <div class="input-group">
        <label for="column-select">in Column:</label>
        <select
          id="column-select"
          bind:value={$metadata.column}
          class="select-field"
        >
          {#each $preferencesStore.columns as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </div>
    {/if}
  </div>
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
