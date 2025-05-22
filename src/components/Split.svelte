<script lang="ts">
  import Table from "./results/Table.svelte";
  import Filters from "./results/filterSwitch.svelte";
  import RemoveBar from "./results/removeBar.svelte";
  import Toolbar from "./results/Toolbar.svelte";
  import RemoveButton from "./results/removeButton.svelte";
  import Status from "./Status.svelte";
  import Form from "./registration/Form.svelte";
  import Algorithms from "./search/Algorithms.svelte";
  import SearchButton from "./search/SearchButton.svelte";
  import Metadata from "./Metadata.svelte";
  import MetadataButton from "./metadata/Button.svelte";
  import MetadataFields from "./metadata/Fields.svelte";

  import { preferencesStore } from "../stores/preferences";
  import { showStatus } from "../stores/status";
  import { isRegistered } from "../stores/registration";

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
      {#if $isRegistered}
        <Filters />
      {:else}
        <h2>
          Search Results:
          <span class="basic-text" style="display: inline-flex; margin: 0;">
            {total} duplicates found
          </span>
        </h2>
      {/if}
      <RemoveButton />
    </div>

    {#if $isRegistered}
      {#if $preferencesStore.showToolbars}
        <Toolbar />
      {/if}
      <div class="block inner">
        {#if $showStatus}
          <Status />
        {:else}
          <Table />
        {/if}
      </div>
      {#if $preferencesStore.showToolbars}
        <RemoveBar />
      {/if}
    {:else}
      <p>Registration Required to View Results</p>
      <Form />
    {/if}
  </div>
</div>
