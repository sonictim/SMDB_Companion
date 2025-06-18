<!-- filepath: /Users/tfarrell/Documents/CODE/SMDB_Companion/src/components/ResultsSkinny.svelte -->
<script lang="ts">
  import {
    selectedItemsStore,
    totalChecksStore,
    selectedChecksStore,
  } from "../../stores/results";
  import { metadataStore, replaceMetadata } from "../../stores/metadata";
  import { databaseStore } from "../../stores/database";
  import { removeRecords, removeSelectedRecords } from "../../stores/remove";
  import { isRemove, isFilesOnly } from "../../stores/menu";
  import RegButton from "../registration/Button.svelte";
  import { isRegistered } from "../../stores/registration";
  import { cancelSearch, showStatus } from "../../stores/status";

  import { OctagonX, NotebookPenIcon, X } from "lucide-svelte";
  import type { Registration } from "../../stores/types";
  import { preferencesStore } from "../../stores/preferences";

  $: pref = $preferencesStore;

  $: metadata = $metadataStore;
  $: selectedItems = $selectedItemsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;
</script>

<div style="margin-left: auto; display: flex; gap: 20px;">
  {#if $showStatus}
    <button
      class="cta-button cancel"
      on:click={async () => {
        let result = await cancelSearch();
      }}
    >
      <X size={18} />
      <span>Cancel Search</span>
    </button>
  {:else if $isRegistered}
    {#if $isRemove}
      {#if $isFilesOnly}
        {#if selectedItems.size > 0}
          <button class="cta-button cancel" on:click={removeSelectedRecords}>
            <OctagonX size="18" />
            Remove {selectedChecks} Selected Files
          </button>
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove all {totalChecks} Files
          </button>
        {:else if $preferencesStore.erase_files === "Keep"}
          <button class="cta-button inactive">
            <OctagonX size="18" />
            Remove {totalChecks} Files
          </button>
        {:else}
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove {totalChecks} Files
          </button>
        {/if}
      {:else if selectedItems.size > 0}
        <button class="cta-button cancel" on:click={removeSelectedRecords}>
          <OctagonX size="18" />
          Remove {selectedChecks} Selected Records
        </button>
        <button class="cta-button cancel" on:click={removeRecords}>
          <OctagonX size="18" />
          Remove all {totalChecks} Records
        </button>
      {:else if $databaseStore == null || $databaseStore.name == "" || $databaseStore.name == "Select Database"}
        <button class="cta-button inactive">
          <OctagonX size="18" />
          Remove {totalChecks} Records
        </button>
      {:else}
        <button class="cta-button cancel" on:click={removeRecords}>
          <OctagonX size="18" />
          Remove {totalChecks} Records
        </button>
      {/if}
    {:else}
      <button class="cta-button cancel" on:click={replaceMetadata}>
        <NotebookPenIcon size="18" />
        <span>Replace '{metadata.find}' with '{metadata?.replace || ""}'</span>
      </button>
    {/if}
  {:else}
    <RegButton />
  {/if}
</div>
