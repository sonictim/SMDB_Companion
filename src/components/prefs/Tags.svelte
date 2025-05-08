<script lang="ts">
  import VirtualList from "svelte-virtual-list"; // Ensure this package is installed
  import {
    Square,
    CheckSquare,
    OctagonX,
    ArrowBigLeft,
    ArrowBigRight,
  } from "lucide-svelte";

  import {
    preferencesStore,
    audiosuite_tag_add,
    audiosuite_tag_remove,
    audiosuite_to_filename_tags,
    selected_audiosuite_to_filename_tags,
    filename_tag_add,
    filename_tag_remove,
    filename_to_audiosuite_tags,
    selected_filename_to_audiosuite_tags,
  } from "../../stores/preferences";
  import { writable, get } from "svelte/store";
  let newSelect: string;
  let newTag: string;

  $: pref = $preferencesStore;
  let selectedItems = new Set<string>();
  let selectedTags = new Set<string>();

  // Create separate stores for tracking the last selected indices
  const lastItemIndex = writable<number>(-1);
  const lastTagIndex = writable<number>(-1);

  let isMove: boolean = true;

  function toggleSelected(item: string, index: number, event: MouseEvent) {
    event.preventDefault();
    console.log("toggleSelected called with index:", index);

    const items = Array.from(pref.autoselects);

    if (event.altKey) {
      // Alt key: Select all or none
      if (selectedItems.size > 0) {
        selectedItems.clear();
      } else {
        items.forEach((item) => selectedItems.add(item));
      }
      selectedItems = new Set(selectedItems); // Trigger reactivity
      return;
    }

    // Shift key - range selection
    if (event.shiftKey && $lastItemIndex !== -1) {
      console.log("Shift key detected", index, $lastItemIndex);
      const start = Math.min($lastItemIndex, index);
      const end = Math.max($lastItemIndex, index);

      // Add all items in the range
      for (let i = start; i <= end; i++) {
        if (i < items.length) {
          selectedItems.add(items[i]);
        }
      }
    } else {
      // Regular selection
      if (selectedItems.has(item)) {
        selectedItems.delete(item);
      } else {
        selectedItems.add(item);
        lastItemIndex.set(index); // Store the last selected index
        console.log("Updated lastItemIndex to:", index);
      }
    }

    selectedItems = new Set(selectedItems); // Trigger reactivity
  }

  function toggleTags(item: string, index: number, event: MouseEvent) {
    event.preventDefault();
    console.log("toggleTags called with index:", index);

    const tags = Array.from(pref.tags);

    if (event.altKey) {
      // Alt key: Select all or none
      if (selectedTags.size > 0) {
        selectedTags.clear();
      } else {
        tags.forEach((tag) => selectedTags.add(tag));
      }
      selectedTags = new Set(selectedTags); // Trigger reactivity
      return;
    }

    // Shift key - range selection
    if (event.shiftKey && $lastTagIndex !== -1) {
      console.log("Shift key detected", index, $lastTagIndex);
      const start = Math.min($lastTagIndex, index);
      const end = Math.max($lastTagIndex, index);

      // Add all items in the range
      for (let i = start; i <= end; i++) {
        if (i < tags.length) {
          selectedTags.add(tags[i]);
        }
      }
    } else {
      // Regular selection
      if (selectedTags.has(item)) {
        selectedTags.delete(item);
      } else {
        selectedTags.add(item);
        lastTagIndex.set(index); // Store the last selected index
        console.log("Updated lastTagIndex to:", index);
      }
    }

    selectedTags = new Set(selectedTags); // Trigger reactivity
  }

  function removeSelected(list: string[]) {
    list.forEach((item) => removeSelect(item));
    clearSelected();
  }

  function removeSelect(item: string) {
    filename_tag_remove(item);
  }

  function clearSelected() {
    selectedItems.clear();
    selectedItems = new Set();
    lastItemIndex.set(-1); // Reset index when clearing selection
  }

  function addSelected(item: string) {
    filename_tag_add(item);
    pref.autoselects.sort();
    preferencesStore.set(pref);
    newSelect = "";
  }

  function moveToTags() {
    if (selectedItems.size == 0 && selectedTags.size == 0) {
      filename_to_audiosuite_tags(isMove);
    } else {
      selected_filename_to_audiosuite_tags(selectedItems, isMove);
    }

    clearSelected();
  }

  function moveToSelects() {
    if (selectedItems.size == 0 && selectedTags.size == 0) {
      audiosuite_to_filename_tags(isMove);
    } else {
      selected_audiosuite_to_filename_tags(selectedTags, isMove);
    }
    clearTags();
  }

  function addTag(item: string) {
    audiosuite_tag_add(item);
    pref.tags.sort();
    preferencesStore.set(pref);
    newTag = "";
  }

  function removeTags(list: string[]) {
    list.forEach((item) => removeTag(item));
    clearTags();
  }

  function clearTags() {
    selectedTags.clear();
    selectedTags = new Set();
    lastTagIndex.set(-1); // Reset index when clearing selection
  }

  function removeTag(item: string) {
    audiosuite_tag_remove(item);
  }
</script>

<div class="page-columns">
  <div>
    <div class="block">
      <div class="header">
        <h2>Audiosuite Tags</h2>
        <button
          class="cta-button cancel"
          on:click={() => removeTags([...selectedTags])}
        >
          <OctagonX size="18" />
          Remove
        </button>
      </div>
      <!-- Files with these tags will only be marked for removal if they cannot find a root file with the same name. -->

      <div class="bar">
        <button class="cta-button small" on:click={() => addTag(newTag)}>
          Add
        </button>
        <input
          type="text"
          id="find-text"
          bind:value={newTag}
          placeholder="New Tag"
          class="input-field"
        />
      </div>

      <div class="block inner">
        <VirtualList items={Array.from(pref.tags)} let:item let:index>
          <div
            on:click={(event) => toggleTags(item, index, event)}
            class="list-item"
            class:selected-item={selectedTags.has(item)}
            class:unselected-item={!selectedTags.has(item)}
          >
            {item}
          </div>
        </VirtualList>
      </div>
    </div>
  </div>

  <div class="arrow-column">
    <div class="move-button-container">
      <button class="arrow-button" on:click={() => moveToSelects()}>
        <ArrowBigRight size="100" />
      </button>
      <span>
        {#if selectedItems.size === 0 && selectedTags.size == 0}
          <select
            class="select-field"
            style="font-size: 10px; width: 100px; margin-left: -1px; text-align: center; text-align-last: center;"
            bind:value={isMove}
          >
            <option value={true}>Move All</option>
            <option value={false}>Copy All</option>
          </select>
        {:else}
          <select
            class="select-field"
            style="font-size: 10px; width: 100px; margin-left: -1px; text-align: center; text-align-last: center;"
            bind:value={isMove}
          >
            <option value={true}>Move Selected</option>
            <option value={false}>Copy Selected</option>
          </select>
        {/if}
      </span>
      <button class="arrow-button" on:click={() => moveToTags()}>
        <!-- → -->
        <ArrowBigLeft size="100" />
      </button>
    </div>
  </div>

  <div>
    <div class="block">
      <div class="header">
        <h2>Filename Tags</h2>
        <button
          class="cta-button cancel"
          on:click={() => removeSelected([...selectedItems])}
        >
          <OctagonX size="18" />
          Remove
        </button>
      </div>
      <div class="bar">
        <button
          class="cta-button small"
          on:click={() => addSelected(newSelect)}
        >
          Add
        </button>
        <input
          type="text"
          id="find-text"
          bind:value={newSelect}
          placeholder="Add New String"
          class="input-field"
        />
      </div>

      <div class="block inner">
        <VirtualList items={Array.from(pref.autoselects)} let:item let:index>
          <div
            on:click={(event) => toggleSelected(item, index, event)}
            class="list-item"
            class:selected-item={selectedItems.has(item)}
            class:unselected-item={!selectedItems.has(item)}
          >
            {item}
          </div>
        </VirtualList>
      </div>
    </div>
  </div>
  <!-- <div class="column">Column 3</div> -->
</div>

<style>
  .page-columns {
    display: grid;
    grid-template-columns: 1fr auto 1fr; /* Left, center, right */
    /* gap: 20px; */
  }

  .arrow-column {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .move-button-container {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .arrow-button {
    color: var(--secondary-bg);
    font-size: 2rem;
    background: transparent;
    border: none;
    /* padding: 10px; */
    cursor: pointer;
    /* margin: 5px; */
  }

  .arrow-button:hover {
    /* background-color: #f0a500; */
    color: var(--accent-color);
  }

  .block {
    height: calc(100vh - 160px);
  }

  .list-item {
    user-select: none; /* Prevents text selection */
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }
</style>
