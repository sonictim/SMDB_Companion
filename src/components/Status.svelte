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

<div class="block inner">
  <span>
    <Loader size={24} class="spinner ml-2" style="color: var(--accent-color)" />
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
