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
  import { isRemove } from "../../stores/menu";
  import RegButton from "../registration/Button.svelte";
  import { isRegistered } from "../../stores/registration";

  import { OctagonX, NotebookPenIcon, Loader } from "lucide-svelte";
  import type { Registration } from "../../stores/types";

  $: metadata = $metadataStore;
  $: selectedItems = $selectedItemsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore;
</script>

<div style="margin-left: auto; display: flex; gap: 20px;">
  {#if $isRegistered}
    {#if $isRemove}
      {#if selectedItems.size > 0}
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
