<script lang="ts">
  import { Key, KeyRound, Lock, LockKeyholeOpen } from "lucide-svelte"; // or UserCheck, ShieldCheck
  import { setReg, isRegistered } from "../../stores/registration";
</script>

<button
  class="cta-button cancel"
  style="width: 200px;"
  on:click={async () => {
    const result = await setReg();
    console.log("Registration result:", result);
  }}
>
  <KeyRound size="20" />
  Register
</button>
