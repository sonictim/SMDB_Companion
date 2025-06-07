<script lang="ts">
  import { Loader } from "lucide-svelte";
  import { onMount, onDestroy } from "svelte";
  import MetadataButton from "./metadata/Button.svelte";
  import MetadataFields from "./metadata/Fields.svelte";

  import {
    showStatus,
    searchProgressStore,
    initializeSearchListeners,
  } from "../stores/status";

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

<div class="block" style="height: 100%; margin-top: 20px">
  <div class="header">
    <h2>Metadata Replacement</h2>
    <MetadataButton />
  </div>
  <!-- {#if $showStatus}
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
  {:else} -->
  <MetadataFields />
  <!-- {/if} -->
</div>
