<script lang="ts">
  import {
    X,
    Search,
    AlertCircle,
    Loader,
    Square,
    CheckSquare,
    Smile,
  } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Window } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";
  export let isRegistered: boolean;

  import { registrationStore } from "../store";
  import { resultsStore } from "../session-store";
  import { get } from "svelte/store";

  import type { Registration } from "../store";

  const DEBUG_MODE = import.meta.env.DEV || false; // Will be true in development, false in production
  let attemptFailed = false;

  let reg: Registration = get(registrationStore);

  // let reg: Registration = { name: '', email: '', license: '' };
  $: results = $resultsStore;
  let total = 0;
  let r2 = "";
  let componentReady = false;
  let inputsEnabled = true;

  async function getreg() {
    if (!DEBUG_MODE) return;
    try {
      r2 = await invoke<string>("get_reg", { data: reg }); // Fetch total count first
    } catch (error) {
      console.error("Failed to fetch data:", error);
    }
  }

  async function setReg() {
    attemptFailed = false;
    registrationStore.set(reg);
    invoke<boolean>("check_reg", { data: reg })
      .then((result) => {
        attemptFailed = !result;
        isRegistered = result;
        console.log("Registration:", result);
        if (!result) getreg();
      })
      .catch((error) => console.error(error));
  }

  async function fetchData() {
    total = 0;
    results.forEach((result) => {
      if (!result.algorithm.includes("Keep")) {
        total++;
      }
    });
  }

  onMount(async () => {
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

{#if !componentReady}
  <div class="loading">
    <Loader size={24} />
    <span>Preparing registration form...</span>
  </div>
{:else}
  <div class="block">
    <div class="header">
      <h2>Registration</h2>
      <span>
        <!-- <button class="cta-button" on:click={resetInputs}> Reset Form </button> -->
        <button class="cta-button cancel" on:click={setReg}> Register </button>
      </span>
    </div>
    <div class="input-group2" style="margin-left: 110px;">
      <label for="case-sensitive">
        {#if isRegistered}
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
          bind:value={reg.name}
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
          bind:value={reg.email}
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
          bind:value={reg.license}
          placeholder="Enter License number"
          class="input-field"
          disabled={!inputsEnabled}
        />
      </div>
    </div>

    {#if attemptFailed}
      <p style="margin-left: 110px; color: var(--topbar-color)">
        Registration Attempt Failed. Please double check your credentials.
      </p>
    {/if}

    {#if DEBUG_MODE}
      <div class="debug-info">
        Debug: {r2}
      </div>
    {/if}
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
