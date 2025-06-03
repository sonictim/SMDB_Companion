<script lang="ts">
  import { SearchCode } from "lucide-svelte";
  import { databaseStore } from "../../stores/database";
  import { showStatus } from "../../stores/status";
  $: database = $databaseStore;

  import { metadataStore, findMetadata } from "../../stores/metadata";

  $: metadata = metadataStore;

  async function searchForMetadata() {
    showStatus.set(true);
    await findMetadata();
    showStatus.set(false);
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
