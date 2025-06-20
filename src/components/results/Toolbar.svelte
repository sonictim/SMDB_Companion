<script lang="ts">
  import {
    selectedItemsStore,
    clearSelected,
    invertSelected,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
  } from "../../stores/results";
  import { Loader } from "lucide-svelte";

  $: selectedItems = $selectedItemsStore;

  let processingBatch = false;

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    setTimeout(() => {
      try {
        checkSelected();
      } finally {
        processingBatch = false;
      }
    }, 10);
  }
</script>

<div class="bar" style="margin-top: 0px">
  <div class="left-side">
    <button class="small-button" on:click={toggleChecksSelected}
      >Toggle Selected</button
    >
    <button class="small-button" on:click={checkSelectedWithIndicator}
      >Check Selected</button
    >
    <button class="small-button" on:click={uncheckSelected}
      >Uncheck Selected</button
    >
    <button class="small-button" on:click={invertSelected}
      >Invert Selections</button
    >
    <button class="small-button" on:click={clearSelected}
      >Clear Selections</button
    >
    <!-- {#if selectedItems.size > 0}
      <p style="margin-left: 10px">({selectedItems.size} selected)</p>
    {/if} -->
    {#if processingBatch}
      <div class="batch-processing">
        <Loader size={24} class="spinner" />
        <span>Processing {selectedItems.size} items...</span>
      </div>
    {/if}
  </div>

  <!-- This slot allows content to be injected from the parent -->
  <div class="right-side">
    <slot name="right"></slot>
  </div>
</div>

<style>
  .left-side {
    display: flex;
    align-items: center;
  }

  .right-side {
    display: flex;
    align-items: center;
    margin-left: auto;
  }

  .right-side:empty {
    display: none;
  }
</style>
