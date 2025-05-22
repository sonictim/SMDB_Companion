<script lang="ts">
  import { SearchCode } from "lucide-svelte";
  import { databaseStore } from "../../stores/database";
  $: database = $databaseStore;

  let isFinding = false;

  import { metadataStore, findMetadata } from "../../stores/metadata";

  $: metadata = metadataStore;

  function searchForMetadata() {
    isFinding = true;
    findMetadata();
    isFinding = false;
  }
</script>

{#if database == null || database.name == "" || database.name == "Select Database" || $metadata.find == "" || $metadata.find == null}
  <button class="cta-button inactive">
    <SearchCode size={18} />
    <span> Find Metadata </span>
  </button>
{:else}
  <button class="cta-button" on:click={searchForMetadata}>
    <SearchCode size={18} />
    <span> Find Metadata </span>
  </button>
{/if}
