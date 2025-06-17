<script lang="ts">
  import { X, Search, SearchCheck, SearchCode } from "lucide-svelte";
  import { databaseStore } from "../../stores/database";
  $: database = $databaseStore;

  import { preferencesStore } from "../../stores/preferences";
  import {
    showStatus,
    toggleSearch, // Import the moved functions
  } from "../../stores/status";
  import { showSearchPopup } from "../../stores/menu";

  // Create a reactive variable that updates whenever preferences change
  $: anyAlgorithmEnabled = (() => {
    // First check if basic is enabled
    const basicAlgorithm = $preferencesStore?.algorithms?.find(
      (algo) => algo.id === "basic"
    );
    const isBasicEnabled = basicAlgorithm?.enabled || false;

    // These are the algorithms that depend on "basic" being enabled
    const dependentAlgorithms = ["audiosuite", "filename"];

    // Check if any algorithm is enabled
    return (
      $preferencesStore?.algorithms?.some((algo) => {
        // If this is a dependent algorithm, it only counts if basic is also enabled
        if (dependentAlgorithms.includes(algo.id)) {
          return algo.enabled && isBasicEnabled;
        }
        // All other algorithms work independently
        return algo.enabled;
      }) || false
    );
  })();
</script>

{#if database == null || database.name == "" || database.name == "Select Database" || !anyAlgorithmEnabled}
  <button class="cta-button inactive">
    <SearchCheck size={18} />
    <span>Search for Records</span>
  </button>
{:else if $showStatus}
  <button
    class="cta-button cancel"
    on:click={async () => {
      let result = await toggleSearch();
    }}
  >
    <X size={18} />
    <span>Cancel</span>
  </button>
{:else}
  <button
    class="cta-button"
    on:click={async () => {
      $showSearchPopup = false; // Close the search popup
      let result = await toggleSearch();
    }}
  >
    <SearchCheck size={18} />
    <span>Search for Records</span>
  </button>
{/if}
