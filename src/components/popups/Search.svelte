<script lang="ts">
  import SearchButton from "../search/SearchButton.svelte";
  import Algorithms from "../search/Algorithms.svelte";
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block" style="height: 60%">
    <SearchButton />

    <div
      class="grid"
      style="grid-template-columns: repeat(1, 1fr);
              gap: 0.5rem;  
              margin-top: 20px;"
    >
      <Algorithms />
    </div>
  </div>
</div>
