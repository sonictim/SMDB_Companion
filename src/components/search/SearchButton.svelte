<script lang="ts">
  import { X, Search, SearchCheck, SearchCode } from "lucide-svelte";
  import { databaseStore } from "../../stores/database";
  $: database = $databaseStore;

  import { preferencesStore } from "../../stores/preferences";
  import {
    showStatus,
    toggleSearch, // Import the moved functions
  } from "../../stores/status";

  $preferencesStore?.algorithms?.find((a: { id: string }) => a.id === "basic")
    ?.enabled || false;

  function checkAnyAlgorithmEnabled() {
    return $preferencesStore.algorithms.some((algo) => algo.enabled);
  }
</script>

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
