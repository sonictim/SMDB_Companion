<script lang="ts">
  import Table from "./results/Table.svelte";
  import Filters from "./results/filterSwitch.svelte";
  import RemoveBar from "./results/removeBar.svelte";
  import Toolbar from "./results/Toolbar.svelte";
  import RemoveButton from "./results/removeButton.svelte";
  import Status from "./Status.svelte";
  import Form from "./registration/Form.svelte";
  import Algorithms from "./search/Algorithms.svelte";
  import Button from "./search/SearchButton.svelte";
  import { preferencesStore } from "../stores/preferences";
  import { showStatus } from "../stores/status";
  import { isRegistered } from "../stores/registration";

  let total = 0;
</script>

<div class="grid">
  <div class="page-columns">
    <div class="block">
      <Button />

      <div
        class="grid"
        style="grid-template-columns: repeat(1, 1fr);
                gap: 0.5rem;
                margin-top: 20px;"
      >
        <Algorithms />
      </div>
    </div>
  </div>
  <div class="block" style="width: 75vw;">
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
      <div class="block inner" style="margin-bottom: 15px;">
        {#if $showStatus}
          <Status />
          <!-- {:else if loading}
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
          </div> -->
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
