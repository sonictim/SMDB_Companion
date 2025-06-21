<script lang="ts">
  import { Smile } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Window } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";

  import type { Registration } from "../../stores/types";
  import {
    registrationStore,
    regInput,
    attemptFailed,
  } from "../../stores/registration";
  import { resultsStore } from "../../stores/results";
  import { get } from "svelte/store";
  import { isRegistered } from "../../stores/registration";

  const DEBUG_MODE = import.meta.env.DEV || false;

  // Create a local variable to bind inputs to
  let localReg: Registration = { name: "", email: "", license: "" };

  // Initialize it from the store and keep it synchronized
  onMount(() => {
    // Initialize from regInput store
    const storedInput = get(regInput);
    localReg = { ...storedInput };
  });

  // Watch for changes to localReg and update the store
  $: if (componentReady) {
    regInput.set({ ...localReg });
  }

  // Update each field individually to ensure the store is updated on each change
  async function updateField(
    field: "name" | "email" | "license",
    value: string
  ) {
    localReg[field] = value;
    regInput.update((current) => ({
      ...current,
      [field]: value,
    }));

    // Update debug display whenever a field changes
    if (DEBUG_MODE) {
      await getreg();
    }
  }

  $: results = $resultsStore;
  let total = 0;
  let r2 = "";
  let componentReady = false;
  let inputsEnabled = true;

  // Add reactivity to update debug display whenever localReg changes
  $: if (DEBUG_MODE && componentReady) {
    getreg();
  }

  async function getreg() {
    if (!DEBUG_MODE) return;
    try {
      // Get the most current data from both local state and store
      const currentLocalData = { ...localReg };
      const currentStoreData = get(regInput);

      console.log("Debug - Local data:", currentLocalData);
      console.log("Debug - Store data:", currentStoreData);

      // Use the local data for the debug display
      r2 = await invoke<string>("get_reg", { data: currentLocalData });

      // If the invoke fails to update, at least show the current input state
      if (!r2) {
        r2 = JSON.stringify(currentLocalData, null, 2);
      }
    } catch (error) {
      console.error("Failed to fetch data:", error);
      // Still show something even if the backend call fails
      r2 = JSON.stringify(localReg, null, 2);
    }
  }

  async function fetchData() {
    total = 0;
    results.forEach((resultArray) => {
      resultArray.forEach((result) => {
        if (!result.algorithm.includes("Keep")) {
          total++;
        }
      });
    });
  }

  onMount(async () => {
    // Get stored registration info from both stores
    const storedReg = get(registrationStore);
    const storedInput = get(regInput);

    // Prioritize regInput values if they exist, otherwise use registrationStore values
    localReg = {
      name: storedInput.name || storedReg.name || "",
      email: storedInput.email || storedReg.email || "",
      license: storedInput.license || storedReg.license || "",
    };

    // Update the regInput store with our local values to ensure consistency
    regInput.set({ ...localReg });

    attemptFailed.set(false);

    try {
      // Ensure the window is focused
      await Window.getCurrent().setFocus();

      // Small delay to ensure the webview is ready
      await new Promise((resolve) => setTimeout(resolve, 100));

      // Run your existing init functions
      await fetchData();
      await getreg();

      // Mark component as ready
      componentReady = true;

      // Force a DOM update
      await tick();

      // Try to focus the first input field
      document.getElementById("name-input")?.focus();
    } catch (error) {
      console.error("Error initializing registration component:", error);
      // If initialization fails, still mark as ready to show error state
      componentReady = true;
    }
  });

  // Add a function to manually focus an input if clicking isn't working
  function focusInput(id: string) {
    document.getElementById(id)?.focus();
  }

  // Add a function to toggle input state in case of issues
  function resetInputs() {
    inputsEnabled = false;
    setTimeout(() => {
      inputsEnabled = true;
      tick().then(() => {
        document.getElementById("name-input")?.focus();
      });
    }, 50);
  }
  import { openUrl } from "@tauri-apps/plugin-opener";

  async function openPurchaseLink() {
    await openUrl("https://buy.stripe.com/9AQcPw4D0dFycSYaEE");
  }
</script>

<div class="input-group2" style=" margin-top: 40px;">
  <label for="case-sensitive">
    {#if $isRegistered}
      <Smile style="color: var(--topbar-color)" />
      Succesfully registered! Thank you for your support.
    {:else}
      <p>Please enter your credentials below:</p>
      <span
        >If you have not yet purchased a license, you can do so by clicking: <button
          class="cta-button small"
          on:click={openPurchaseLink}>HERE</button
        ></span
      >
    {/if}
  </label>
</div>
<div class="input-group">
  <label for="name-input">Name:</label>
  <div class="input-wrapper" on:click={() => focusInput("name-input")}>
    <input
      type="text"
      id="name-input"
      bind:value={localReg.name}
      on:input={(e) => updateField("name", e.currentTarget.value)}
      on:blur={(e) => updateField("name", e.currentTarget.value.trim())}
      placeholder="Enter Registration Name"
      class="input-field"
      disabled={!inputsEnabled}
    />
  </div>
</div>

<div class="input-group">
  <label for="email-input">Email:</label>
  <div class="input-wrapper" on:click={() => focusInput("email-input")}>
    <input
      type="text"
      id="email-input"
      bind:value={localReg.email}
      on:input={(e) => updateField("email", e.currentTarget.value)}
      on:blur={(e) => updateField("name", e.currentTarget.value.trim())}
      placeholder="Enter Registration Email"
      class="input-field"
      disabled={!inputsEnabled}
    />
  </div>
</div>

<div class="input-group">
  <label for="license-input">License:</label>
  <div class="input-wrapper" on:click={() => focusInput("license-input")}>
    <input
      type="text"
      id="license-input"
      bind:value={localReg.license}
      on:input={(e) => updateField("license", e.currentTarget.value)}
      on:blur={(e) => updateField("name", e.currentTarget.value.trim())}
      placeholder="Enter License number"
      class="input-field"
      disabled={!inputsEnabled}
    />
  </div>
</div>

{#if $attemptFailed}
  <p style="margin-left: 70px; color: var(--topbar-color)">
    Registration Attempt Failed. Please double check your credentials.
  </p>
  <p style="margin-left: 70px; color: var(--topbar-color)">
    Consider copy and pasting each field from the email you received after
    purchase.
  </p>
{/if}

{#if DEBUG_MODE}
  <div class="debug-info">
    <div>Debug Response: {r2}</div>
    <div>Form Values: {JSON.stringify(localReg)}</div>
    <div>Store Values: {JSON.stringify($regInput)}</div>
  </div>
{/if}

<style>
  .loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    gap: 1rem;
  }

  .input-wrapper {
    position: relative;
    flex: 1;
    cursor: text;
  }

  /* Make sure inputs take full width of their container */
  .input-field {
    width: 100%;
    z-index: 1; /* Ensure inputs are above any other elements */
  }
  .debug-info {
    margin-top: 8px;
    padding: 8px;
    color: black;
    background-color: #ffe8e8;
    border: 1px dashed #ff5252;
    font-family: monospace;
    font-size: 0.9em;
  }
</style>
