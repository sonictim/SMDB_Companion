<script lang="ts">
  import {
    searchProgressStore,
    initializeSearchListeners,
  } from "../stores/status";
  import { preferencesStore } from "../stores/preferences";
  import { onMount } from "svelte";
  import { Loader } from "lucide-svelte";
  import { algoEnabled } from "../stores/algorithms";
  import { isFilesOnly } from "../stores/menu";

  onMount(() => {
    initializeSearchListeners();
  });
</script>

<div class="block" style="height: 100%;">
  <div
    style="display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; margin-bottom: 10px;"
  >
    <div style="display: flex; align-items: center; gap: 0.5rem;">
      <h2 style="margin: 0;">
        <Loader
          size={24}
          class="spinner ml-2"
          style="color: var(--accent-color)"
        />Search Status:
      </h2>
      <span class="ellipsis"> {$searchProgressStore.searchStage} </span>
    </div>

    {#if $preferencesStore.store_waveforms}
      {#if isFilesOnly}
        <h3 class="tooltip-trigger" style="margin: 0;">
          Progress Save Unavailable
          <span
            class="tooltip-text"
            style="position: absolute; top: 100%; left: 50%; transform: translateX(-50%); margin-top: 5px; height: 60px;"
          >
            There is no database to save progress too. Consider saving your
            results from the File Menu.
          </span>
        </h3>
      {:else if algoEnabled("dual_mono") || algoEnabled("waveform")}
        <h3 class="tooltip-trigger" style="margin: 0;">
          Progress Save Enabled
          <span
            class="tooltip-text"
            style="position: absolute; top: 100%; left: 50%; transform: translateX(-50%); margin-top: 5px; height: 60px;"
          >
            If you abort the processing, running the search again will continue
            from where it left off.
          </span>
        </h3>
      {/if}
    {/if}
  </div>
  <!-- rest of your component stays the same -->
  <div class="block inner">
    <div style="margin-left: 10px; margin-right: 10px; margin: 10px">
      <span style="margin-top: 10px;">
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
  </div>
</div>
