<script lang="ts">
  import { Square, CheckSquare } from "lucide-svelte";
  import { preferencesStore } from "../../stores/preferences";
  import { metadataStore } from "../../stores/metadata";

  $: metadata = metadataStore;

  function toggleCaseSensitivity() {
    metadataStore.update((meta) => ({
      ...meta,
      case_sensitive: !meta.case_sensitive,
    }));
  }
</script>

<div class="input-group2" style="margin-top: 20px">
  <label for="case-sensitive">
    <button type="button" class="grid item" on:click={toggleCaseSensitivity}>
      {#if $metadata.case_sensitive}
        <CheckSquare size={20} class="checkbox checked" />
      {:else}
        <Square size={20} class="checkbox" />
      {/if}
      <span>Case Sensitive</span>
    </button>
  </label>
</div>

<div class="input-group">
  <label for="find-text">Find:</label>
  <input
    type="text"
    id="find-text"
    bind:value={$metadata.find}
    placeholder="Enter text to find"
    class="input-field"
  />
</div>

<div class="input-group">
  <label for="replace-text">Replace:</label>
  <input
    type="text"
    id="replace-text"
    bind:value={$metadata.replace}
    placeholder="Enter text to replace"
    class="input-field"
  />
</div>

<div class="input-group">
  <label for="column-select">in Column:</label>
  <select id="column-select" bind:value={$metadata.column} class="select-field">
    {#each $preferencesStore.columns as option}
      <option value={option}>{option}</option>
    {/each}
  </select>
</div>
