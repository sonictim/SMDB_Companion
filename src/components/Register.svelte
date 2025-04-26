<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  export let isRegistered: boolean;

  import type { Registration } from "../stores/types";
  import { registrationStore } from "../stores/registration";
  import { resultsStore } from "../stores/results";
  import { get } from "svelte/store";

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
    <h2>
      Search Results:
      <span class="basic-text" style="display: inline-flex; margin: 0;">
        {total} duplicates found
      </span>
    </h2>
    <button class="cta-button cancel" on:click={setReg}> Register </button>
  </div>
  <div class="input-group2" style="margin-left: 110px;">
    <label for="case-sensitive">
      <span>
        Registration Required to View Results. License can be purchased:
        <button class="cta-button small" on:click={openPurchaseLink}
          >HERE</button
        ></span
      >
      <p>Please enter your credentials below:</p>
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

  {#if DEBUG_MODE}
    <div class="debug-info">
      Debug: {r2}
    </div>
  {/if}
  {#if attemptFailed}
    <p style="margin-left: 110px;">
      Registration Attempt Failed. Please double check your credentials.
    </p>
  {/if}
  {#if isRegistered}
    <p style="margin-left: 110px;">
      Succesfully Registered! Thank you for your support.
    </p>
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
