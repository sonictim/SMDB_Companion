<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import Table from "../results/Table.svelte";
  import { preferencesStore } from "../../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    enableSelectionsStore,
    clearSelected,
    invertSelected,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
  } from "../../stores/results";
  import { metadataStore } from "../../stores/metadata";
  import { viewStore, showSearchView } from "../../stores/menu";
  import { ask, message } from "@tauri-apps/plugin-dialog";

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: enableSelections = $enableSelectionsStore;

  let processing = false;
  let loading = true;

  let processingBatch = false;
  let loadingResults = true;
  let showLoadingOverlay = true;

  let containerElement: HTMLElement;
  let containerWidth = 0;

  async function replaceMetadata() {
    const confirmed = await ask("Are you sure? This is NOT undoable", {
      title: "⚠️ Confirm Replace",
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });

    if (confirmed && metadata.find && metadata.replace) {
      await invoke("replace_metadata", {
        data: metadata,
      })
        .then(() => {
          console.log("Successfully replaced metadata");
          metadata.find = "";
          metadata.replace = "";
          results = [];
          showSearchView;
        })
        .catch((error) => {
          console.error("Error replacing metadata:", error);
        });
    }
  }

  async function fetchData() {
    try {
      loading = true;
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    setTimeout(() => {
      loading = false;
    }, 100);

    if (containerElement) {
      containerWidth = containerElement.clientWidth;
    }

    activateResultsTab();
  });

  function toggleMarkDirty() {
    metadataStore.update((p) => ({
      ...p,
      mark_dirty: !p.mark_dirty,
    }));
  }

  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let removeStage = "";
  let unlistenRemoveFn: () => void;

  onMount(async () => {
    unlistenRemoveFn = await listen<{
      progress: number;
      message: string;
      stage: string;
    }>("remove-status", (event) => {
      const status = event.payload;
      removeProgress = status.progress;
      removeMessage = status.message;
      removeStage = status.stage;
      console.log(
        `Remove status: ${status.stage} - ${status.progress}% - ${status.message}`
      );
      if (status.stage === "complete") {
        processing = false;
        fetchData();
      }
    });
  });

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });

  import {
    CheckSquare,
    Square,
    SquareEqual,
    NotebookPenIcon,
    OctagonX,
    Loader,
    Copy,
    FileX2,
    Tag,
    AudioWaveform,
    Clock,
    GitCompareArrowsIcon,
    Hash,
    ShieldCheck,
    Search,
    Activity,
    ArrowLeftRight,
  } from "lucide-svelte";
  function getAlgorithmIcon(algoName: string) {
    const iconMap: Record<
      string,
      { component: any; tooltip: string; color?: string }
    > = {
      Keep: {
        component: ShieldCheck,
        tooltip: "Keep",
        color: "var(--success-color)",
      },
      Basic: { component: Copy, tooltip: "Duplicate Match" },
      InvalidPath: { component: FileX2, tooltip: "Invalid Path" },
      SimilarFilename: {
        component: Search,
        tooltip: "Similar Filename",
      },
      Tags: { component: Tag, tooltip: "Duplicate contains Tag" },
      FileTags: { component: Tag, tooltip: "Filename contains tag" },
      Waveforms: { component: AudioWaveform, tooltip: "Waveform Match" },
      Duration: { component: Clock, tooltip: "Duration Match" },
      Compare: { component: GitCompareArrowsIcon, tooltip: "Database Compare" },
      SimilarAudio: { component: Activity, tooltip: "Similar Audio" },
      ExactPCM: { component: AudioWaveform, tooltip: "Exact PCM Hash" },
      DualMono: { component: SquareEqual, tooltip: "Dual Mono" },
      Replace: { component: ArrowLeftRight, tooltip: "Replace Metadata" },
      Remove: {
        component: OctagonX,
        tooltip: "Marked for Removal",
        color: "var(--error-color)",
      },
    };

    return iconMap[algoName] || { component: Hash, tooltip: algoName };
  }

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    setTimeout(() => {
      try {
        checkSelected();
      } finally {
        processingBatch = false;
      }
    }, 10);
  }

  function activateResultsTab() {
    loadingResults = true;
    showLoadingOverlay = true;

    setTimeout(() => {
      const timer = setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);

      return () => clearTimeout(timer);
    }, 100);
  }

  $: {
    if (filteredItems && filteredItems.length > 0 && loadingResults) {
      setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);
    }
  }
</script>

<div class="block">
  <div class="header">
    <h2>Search Results:</h2>
    <span style="font-size: 18px">
      {results.length} Records found
    </span>

    <div style="margin-left: auto; display: flex; gap: 20px;">
      <button class="cta-button cancel" on:click={replaceMetadata}>
        <NotebookPenIcon size="18" />
        <span>Replace '{metadata.find}' with '{metadata?.replace || ""}'</span>
      </button>
    </div>
  </div>
  {#if $preferencesStore.showToolbars}
    <div
      class="bar"
      style="margin-top: 10px; margin-bottom: 20px; padding: 0px;"
    >
      {#if enableSelections}
        <button class="small-button" on:click={toggleChecksSelected}
          >Toggle Selected</button
        >
        <button class="small-button" on:click={checkSelectedWithIndicator}
          >Check Selected</button
        >
        <button class="small-button" on:click={uncheckSelected}
          >Uncheck Selected</button
        >
        <button class="small-button" on:click={invertSelected}
          >Invert Selections</button
        >
        <button class="small-button" on:click={clearSelected}
          >Clear Selections</button
        >
        {#if selectedItems.size > 0}
          <p style="margin-left: 10px">({selectedItems.size} selected)</p>
        {/if}
        {#if processingBatch}
          <div class="batch-processing">
            <Loader size={24} class="spinner" />
            <span>Processing {selectedItems.size} items...</span>
          </div>
        {/if}
      {/if}

      <div class="filter-container">
        <button
          type="button"
          class="grid item"
          style="margin-left: 120px"
          on:click={toggleMarkDirty}
        >
          {#if $metadataStore.mark_dirty}
            <CheckSquare
              size={20}
              class="checkbox checked {metadata.column == 'FilePath' ||
              metadata.column == 'Filename' ||
              metadata.column == 'Pathname'
                ? 'inactive'
                : ''}"
            />
          {:else}
            <Square size={20} class="checkbox" />
          {/if}
          <span
            class={metadata.column == "FilePath" ||
            metadata.column == "Filename" ||
            metadata.column == "Pathname"
              ? "inactive"
              : ""}>Mark Records as Dirty</span
          >
        </button>
      </div>
    </div>
  {/if}
  <div
    class="block inner"
    bind:this={containerElement}
    style="margin-bottom: 15px;"
  >
    {#if loading}
      <p class="ellipsis">Loading data...</p>
    {:else if processing}
      <div class="block inner">
        <span>
          <Loader
            size={24}
            class="spinner ml-2"
            style="color: var(--accent-color)"
          />
          {removeMessage}
        </span>
        <div class="progress-container">
          <div class="progress-bar" style="width: {removeProgress}%"></div>
        </div>
      </div>
    {:else}
      <Table />
    {/if}
  </div>
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

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    background-color: var(--secondary-bg);
    margin-bottom: 10px;
  }

  .header h2 {
    margin: 0;
  }

  .spinner {
    animation: spin 1.5s linear infinite;
    color: var(--accent-color);
  }

  @keyframes spin {
    100% {
      transform: rotate(360deg);
    }
  }
</style>
