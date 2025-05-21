<script lang="ts">
  import { setReg, isRegistered } from "../../stores/registration";
</script>

<button
  class="cta-button cancel"
  on:click={async () => {
    const result = await setReg();
    console.log("Registration result:", result);
  }}
>
  Register
</button>
