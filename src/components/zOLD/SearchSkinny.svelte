<script lang="ts">
  import Algorithms from "./search/Algorithms.svelte";
  import Button from "./search/SearchButton.svelte";
</script>

<div class="page-columns">
  <div class="block">
    <Button />

    <div class="grid">
      <Algorithms />
    </div>

    <span style="margin-left: 255px"> </span>
  </div>
</div>

<style>
  .grid {
    grid-template-columns: repeat(1, 1fr);
    /* margin-top: -10px; */
    gap: 0.5rem;
  }

  .page-columns {
    display: grid;
    grid-template-columns: repeat(1, 1fr); /* 3 equal columns */
    gap: 10px;
  }

  :global(.checkbox.checked) {
    color: var(--accent-color);
    min-width: 20px;
    min-height: 20px;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
    min-width: 20px;
    min-height: 20px;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .block {
    padding: 20px 20px;
    border-radius: 8px;
    margin-bottom: 0px;
    height: calc(100vh - 110px);
    width: 20vw;
  }
</style>
