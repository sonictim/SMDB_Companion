<script lang="ts">
  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    TriangleAlert,
    Loader,
    X,
  } from "lucide-svelte";
  import Table from "./results/Table.svelte";
  import Filters from "./results/filterSwitch.svelte";
  import RemoveBar from "./results/removeBar.svelte";
  import Toolbar from "./results/Toolbar.svelte";
  import RemoveButton from "./results/removeButton.svelte";
  import Status from "./Status.svelte";
  import Form from "./registration/Form.svelte";
  import RegButton from "./registration/Button.svelte";

  import Algorithms from "./search/Algorithms.svelte";
  import SearchButton from "./search/SearchButton.svelte";
  import Metadata from "./Metadata.svelte";
  import MetadataButton from "./metadata/Button.svelte";
  import MetadataFields from "./metadata/Fields.svelte";
  import ResultsComponent from "./Results.svelte";

  import { preferencesStore } from "../stores/preferences";
  import { showStatus, cancelSearch } from "../stores/status";
  import { isRegistered } from "../stores/registration";
  import { isRemove, RemovePopup } from "../stores/menu";
  import {
    resultsStore,
    filteredItemsStore,
    totalChecksStore,
    countDualMonoFiles,
    selectedItemsStore,
  } from "../stores/results";

  $: totalChecks = $totalChecksStore;

  let total = 0;
</script>

<div class="grid">
  <div class="page-columns">
    <div class="block" style="height: 60%">
      <SearchButton />

      <div
        class="grid"
        style="grid-template-columns: repeat(1, 1fr);
                gap: 0.5rem;  
                margin-top: 20px;"
      >
        <Algorithms />
      </div>
    </div>
    <div class="block" style="gap: 10px; margin-top: 10px; height: 39%">
      <MetadataButton />
      <MetadataFields />
    </div>
  </div>
  <div
    class="block"
    style="width: 75vw; height: calc(100vh - (var(--font-size) * 3));"
  >
    <div class="header">
      <div class="container">
        <div class="left-group">
          {#if $resultsStore.length > 0}
            <h2>Results:</h2>
            <span style="font-size: var(--font-size-lg)">
              {#if $isRemove}
                <h2>{totalChecks}</h2>
                Records marked for Removal
                <h2>{countDualMonoFiles()}</h2>
                Records marked as Dual Mono
                {#if $selectedItemsStore.size > 0}
                  <h2>{$selectedItemsStore.size}</h2>
                  Records Selected
                {/if}
              {:else}
                <h2>{$resultsStore.length}</h2>
                Records found
              {/if}
            </span>
          {/if}
        </div>
      </div>
      {#if $isRegistered}
        {#if $showStatus}
          <button
            class="cta-button cancel"
            on:click={async () => {
              let result = await cancelSearch();
            }}
          >
            <X size={18} />
            <span>Cancel</span>
          </button>
        {:else}
          <button class="cta-button cancel" on:click={() => RemovePopup()}>
            <OctagonX size="18" />
            <!-- Process {totalChecks + countDualMonoFiles()} Records -->
            Process Records
          </button>
        {/if}
      {:else}
        <RegButton />
      {/if}
    </div>
    {#if $isRegistered}
      {#if $preferencesStore.showToolbars}
        <span>
          <Toolbar>
            <div slot="right">
              <Filters />
            </div>
          </Toolbar>
        </span>
      {/if}
      <div class="block inner">
        {#if $showStatus}
          <Status />
        {:else}
          <Table />
        {/if}
      </div>
      <!-- {#if $preferencesStore.showToolbars}
        <RemoveBar />
      {/if} -->
    {:else}
      <span>
        <h2 style="margin-top: 20px; display: inline">
          Registration Required to View Detailed Results or Process Records
        </h2>
        <!-- <p style="display: inline">(But Search is Fully Functional)</p> -->
      </span>
      <Form />
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

  .results-container {
    display: flex;
    flex-direction: column;
    height: calc(
      100vh - (var(--font-size) * 3)
    ); /* Adjust the 60px based on your header height */
  }

  .results-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 300px; /* Minimum height to ensure it's always visible */
  }

  /* This ensures the Table component knows it should fill the space */
  .results-content :global(.virtualized-table-container) {
    flex: 1;
    height: 100% !important;
    min-height: 250px;
  }

  .container {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .left-group {
    display: flex;
    align-items: center;
    gap: 0.5rem; /* Space between h2 and span */
  }
  h2 {
    display: inline;
  }
</style>
