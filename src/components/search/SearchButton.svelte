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
{:else if $showStatus}
  <button class="cta-button cancel">
    <div class="cta-button">
      <X size={18} />
      <span>Cancel</span>
    </div>
  </button>
{:else}
  <button class="cta-button">
    <SearchCheck size={18} />
    <span>Search for Records</span>
  </button>
{/if}
