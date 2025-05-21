<script lang="ts">
  import { Loader } from "lucide-svelte";
  import Form from "./registration/Form.svelte";
  import Button from "./registration/Button.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Window } from "@tauri-apps/api/window";
  import { onMount, tick } from "svelte";

  import type { Registration } from "../stores/types";
  import { registrationStore } from "../stores/registration";
  import { resultsStore } from "../stores/results";
  import { get } from "svelte/store";
  import { isRegistered } from "../stores/registration";

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
        isRegistered.set(result);
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
      <Button />
    </div>
    <Form />
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
