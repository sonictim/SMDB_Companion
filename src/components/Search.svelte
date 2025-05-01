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
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { databaseStore, openSqliteFile } from "../stores/database";
  $: database = $databaseStore;
  import { listen } from "@tauri-apps/api/event";

  // Define props
  // export let dbSize: number;
  export let activeTab: string; // This prop is now bindable
  export let isRemove: boolean;

  // Import viewStore and view control functions
  import { viewStore, showResultsView } from "../stores/menu";

  // Bind to the viewStore values instead of using local props
  $: searchView = $viewStore.searchView;
  $: resultsView = $viewStore.resultsView;

  let isFinding = false;

  import type { Algorithm, Preferences, FileRecord } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import { resultsStore } from "../stores/results";
  import { metadataStore } from "../stores/metadata";
  import {
    isSearching,
    searchProgressStore,
    initializeSearchListeners,
    toggleSearch, // Import the moved functions
    search,
    cancelSearch,
  } from "../stores/status";
  import { get } from "svelte/store";
  import { open } from "@tauri-apps/plugin-dialog";
  import { basename, extname } from "@tauri-apps/api/path";

  async function getFilenameWithoutExtension(fullPath: string) {
    const name = await basename(fullPath); // Extracts filename with extension
    const ext = await extname(fullPath); // Extracts extension
    return name.replace(ext, ""); // Removes extension
  }

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

  // let pref: Preferences = get(preferencesStore);

  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  function getAlgoClass(algo: { id: string }, algorithms: any[]) {
    if (
      (algo.id === "audiosuite" || algo.id === "filename") &&
      !algorithms.find((a) => a.id === "basic")?.enabled
    ) {
      return "inactive";
    }
    return "";
  }

  async function replaceMetadata() {
    isRemove = false;
    isFinding = true;
    const metaValue = get(metadataStore);
    console.log(
      `Finding: ${metaValue.find}, Replacing: ${metaValue.replace}, Case Sensitive: ${metaValue.case_sensitive}, Column: ${metaValue.column}`
    );

    await invoke<FileRecord[]>("find", {
      find: metaValue.find,
      column: metaValue.column,
      caseSensitive: metaValue.case_sensitive,
      pref: get(preferencesStore),
    })
      .then((result) => {
        console.log(result);
        resultsStore.set(result); // ✅ Store the results in session storage
      })
      .catch((error) => console.error(error));
    isFinding = false;
    activeTab = "results";
  }
  function toggleCaseSensitivity() {
    metadataStore.update((meta) => ({
      ...meta,
      case_sensitive: !meta.case_sensitive,
    }));
  }

  function toggleAlgorithm(id: string) {
    preferencesStore.update((prefs) => ({
      ...prefs,
      algorithms: prefs.algorithms.map((algo) =>
        algo.id === id ? { ...algo, enabled: !algo.enabled } : algo
      ),
    }));
  }

  // Handle search tab navigation after search completion
  $: {
    // When search completes and returns results, navigate to results tab
    if (!$isSearching && $resultsStore.length > 0) {
      activeTab = "results";
    }
  }

  // Setup event listener when component mounts
  onMount(() => {
    // Initialize the listeners only once in the application lifecycle
    initializeSearchListeners().then(() => {
      console.log("Search component mounted, isSearching:", $isSearching);
    });

    // Add debugging to track isSearching changes
    const unsubscribe = isSearching.subscribe((value) => {
      console.log("isSearching changed:", value);
    });

    return () => {
      unsubscribe();
    };
  });

  function getAlgorithmTooltip(id: string): string {
    const tooltips: Record<string, string> = {
      basic: "Finds duplicates by comparing Match Criteria set in Preferences.",
      filename:
        "Will attempt to remove extra letters and numbers (.1.4.21.M.wav) from the filename",
      audiosuite:
        "Searches for Protools Audiosuite tags in the filename and checks for orginal file.",
      duration: "Files less than the set duration will be marked for removal.",
      waveform:
        "Compares audio waveform patterns to find similar sounds.  This may take a while.",
      dbcompare: "Compares against another database to find duplicates.",
      invalidpath: "Files with invalid paths will be marked for removal.",
      filetags:
        "Filenames containting tags in this list will be marked for removal.",
      dual_mono:
        "Files where all channels contain identical audio will be identified.  User can choose to remove extra channels in results panel.",
    };

    return tooltips[id] || "No description available";
  }
  function checkAnyAlgorithmEnabled() {
    return $preferencesStore.algorithms.some((algo) => algo.enabled);
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
          class="cta-button {$isSearching ? 'cancel' : ''}"
          on:click={async () => {
            let result = await toggleSearch();
            if (result) {
              activeTab = "results";
              // Use the showResultsView function from the menu store
              showResultsView();
              resultsView = true;
              console.log(
                "searchView:",
                searchView,
                "resultsView:",
                resultsView
              );
            }
          }}
        >
          <div class="flex items-center gap-2">
            {#if $isSearching}
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
    {#if $isSearching}
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
      <div class="grid">
        {#each $preferencesStore.algorithms as algo}
          <div
            class="grid item {getAlgoClass(algo, $preferencesStore.algorithms)}"
          >
            <button
              type="button"
              class="grid item"
              on:click={() => toggleAlgorithm(algo.id)}
            >
              {#if algo.id === "audiosuite" || algo.id === "filename"}
                <span style="margin-right: 20px;"></span>
              {/if}

              {#if algo.enabled}
                <CheckSquare
                  size={20}
                  class="checkbox {(algo.id === 'audiosuite' ||
                    algo.id === 'filename') &&
                  !isBasicEnabled
                    ? 'inactive'
                    : 'checked'}"
                />
              {:else}
                <Square size={20} class="checkbox inactive" />
              {/if}

              <span
                class="tooltip-trigger {(algo.id === 'audiosuite' ||
                  algo.id === 'filename') &&
                !isBasicEnabled
                  ? 'inactive'
                  : ''}"
              >
                {algo.name}
                <span class="tooltip-text"
                  >{getAlgorithmTooltip(algo.id)}
                </span>
              </span>
            </button>

            {#if algo.id === "dbcompare"}
              {#if algo.db !== null && algo.db !== undefined}
                {#await getFilenameWithoutExtension(algo.db) then filename}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_no_static_element_interactions -->
                  <span class="clickable" on:click={getCompareDb}
                    >{filename}</span
                  >
                {/await}
              {:else}
                <button
                  type="button"
                  class="small-button"
                  on:click={getCompareDb}>Select DB</button
                >
              {/if}
            {/if}

            {#if algo.id === "duration"}
              <input
                type="number"
                min="0"
                step="0.1"
                bind:value={algo.min_dur}
                class="duration-input"
                style="width: 55px; background-color: var(--primary-bg)"
              />
              s
            {/if}
          </div>
        {/each}
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
          on:click={replaceMetadata}
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
