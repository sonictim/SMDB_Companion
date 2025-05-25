<script lang="ts">
  import {
    searchProgressStore,
    initializeSearchListeners,
  } from "../stores/status";
  import { onMount } from "svelte";
  import { Loader } from "lucide-svelte";

  onMount(() => {
    initializeSearchListeners();
  });
</script>

<div class="block" style="height: 100%;">
  <div
    style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 10px;"
  >
    <h2 style="margin: 0;">
      <Loader
        size={24}
        class="spinner ml-2"
        style="color: var(--accent-color)"
      />Search Status:
    </h2>
    <span class="ellipsis"> {$searchProgressStore.searchStage} </span>
  </div>
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
