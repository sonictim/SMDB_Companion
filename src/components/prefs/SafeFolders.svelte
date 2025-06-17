<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { Square, CheckSquare, OctagonX } from "lucide-svelte";
  // Import from main store instead
  import {
    preferencesStore,
    safe_folder_remove,
    addSafeFolder,
  } from "../../stores/preferences";

  let selectedSafeMatches = new Set<string>();

  function toggleSafeMatch(item: string) {
    if (selectedSafeMatches.has(item)) {
      selectedSafeMatches.delete(item);
    } else {
      selectedSafeMatches.add(item);
    }
    selectedSafeMatches = new Set(selectedSafeMatches); // Ensure reactivity
  }

  function removeSafeFolders(list: string[]) {
    list.forEach((item) => safe_folder_remove(item));
    clearSafeMatches();
  }

  function clearSafeMatches() {
    selectedSafeMatches.clear();
    selectedSafeMatches = new Set(); // Ensure reactivity
  }
</script>

<div class="block">
  <div class="header">
    <span class="tooltip-trigger">
      <span class="tooltip-text" style="height: 340%;">
        Files in safe folders are completely left out of any duplicate search.
        Useful for folders that contain files you do not want to protect or
        files you may not mind having as duplicates in your library, such as
        sampler instrument libraries.
      </span>
      <h2>Safe/Ignore Folders</h2>
    </span>
    <span>
      <button
        class="cta-button cancel"
        on:click={() => removeSafeFolders([...selectedSafeMatches])}
      >
        <OctagonX size="18" />
        Remove Selected
      </button>
    </span>
  </div>

  <div class="header">
    <div class="button-group">
      <button class="cta-button small" on:click={addSafeFolder}>Add</button>
    </div>
  </div>
  <div class="block inner">
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <VirtualList items={Array.from($preferencesStore.safe_folders)} let:item>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <div
        on:click={() => toggleSafeMatch(item)}
        class="list-item"
        class:selected-item={selectedSafeMatches.has(item)}
        class:unselected-item={!selectedSafeMatches.has(item)}
      >
        {item}
      </div>
    </VirtualList>
  </div>
</div>
