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
  import { onMount } from "svelte";
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

  onMount(fetchData);
  onMount(getreg);

  import { openUrl } from "@tauri-apps/plugin-opener";

  async function openPurchaseLink() {
    await openUrl("https://buy.stripe.com/9AQcPw4D0dFycSYaEE");
  }
</script>

<div class="block">
  <div class="header">
    <h2>Registration</h2>
    <button class="cta-button cancel" on:click={setReg}> Register </button>
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
    <label for="name">Name:</label>
    <input
      type="text"
      id="find-text"
      bind:value={reg.name}
      placeholder="Enter Registration Name"
      class="input-field"
    />
  </div>
  <div class="input-group">
    <label for="email">Email:</label>
    <input
      type="text"
      id="find-text"
      bind:value={reg.email}
      placeholder="Enter Registration Email"
      class="input-field"
    />
  </div>

  <div class="input-group">
    <label for="Reg">License:</label>
    <input
      type="text"
      id="replace-text"
      bind:value={reg.license}
      placeholder="Enter License number"
      class="input-field"
    />
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

<style>
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
