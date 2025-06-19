<!-- Server Database Connection Window -->
<script lang="ts">
  import { onMount } from "svelte";
  import { get } from "svelte/store";
  import {
    serverStore,
    serverDatabasesStore,
    setDatabase,
  } from "../../stores/database";
  import { preferencesStore } from "../../stores/preferences";
  import { applyColors, applyFontSize } from "../../stores/colors";
  import type { Server } from "../../stores/types";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { showPopup } from "../../stores/menu";

  // Local state for form
  let username = "";
  let password = "";
  let address = "";
  let port = 3306;
  let selectedDatabase = "";
  let availableDatabases: string[] = [];

  // Connection state
  let isConnecting = false;
  let isConnected = false;
  let connectionError = "";
  let isDatabasesLoading = false;

  // Load saved server config on mount
  onMount(async () => {
    // Apply colors and font size from preferences
    const currentPrefs = get(preferencesStore);

    // Apply colors
    if (currentPrefs?.colors) {
      await applyColors(currentPrefs.colors);
    }

    // Apply font size
    if (currentPrefs?.fontSize) {
      await applyFontSize(currentPrefs.fontSize);
    }

    // Load saved server config
    const savedConfig = get(serverStore);
    username = savedConfig.name;
    password = savedConfig.password;
    address = savedConfig.address;
    port = savedConfig.port;

    // Load available databases from session store
    const sessionDatabases = get(serverDatabasesStore);
    if (sessionDatabases.length > 0) {
      availableDatabases = sessionDatabases;
      isConnected = true;
    }
  });

  // Save current form state to store
  function saveToStore() {
    serverStore.set({
      name: username,
      password: password,
      address: address,
      port: port,
    });
  }

  // Test connection and fetch available databases
  async function testConnection() {
    if (!username || !password || !address || !port) {
      connectionError = "Please fill in all connection fields";
      return;
    }

    isConnecting = true;
    connectionError = "";

    try {
      // TODO: Replace with actual Tauri command to test server connection
      // For now, simulate the connection
      let url = `mysql://${username}:${password}@${address}:${port}`;
      console.log("Testing connection to:", url);
      availableDatabases = await invoke<string[]>("test_server_database", {
        url: url,
      });
      console.log("Available databases:", availableDatabases);

      // Save databases to session store
      serverDatabasesStore.set(availableDatabases);

      isConnected = true;
      saveToStore();

      console.log("Connected to server:", address);
    } catch (error) {
      connectionError = `Connection failed: ${error}`;
      isConnected = false;
      availableDatabases = [];
      // Clear session store on error
      serverDatabasesStore.set([]);
    } finally {
      isConnecting = false;
    }
  }

  // Connect to selected database
  async function connectToDatabase() {
    if (
      !selectedDatabase ||
      selectedDatabase === "" ||
      selectedDatabase === "Choose a database..."
    ) {
      connectionError = "Please select a database";
      return;
    }

    isDatabasesLoading = true;
    connectionError = "";

    try {
      const url = `mysql://${username}:${password}@${address}:${port}/${selectedDatabase}`;
      console.log("Connecting to database with URL:", url);

      // Call setDatabase to connect to the specific database
      await setDatabase(url, false);

      console.log("Connected to database:", selectedDatabase);

      showPopup.set(false);
    } catch (error) {
      connectionError = `Database connection failed: ${error}`;
    } finally {
      isDatabasesLoading = false;
    }
  }

  // Clear form and reset connection
  function resetConnection() {
    isConnected = false;
    availableDatabases = [];
    selectedDatabase = "";
    connectionError = "";
    // Clear session store
    serverDatabasesStore.set([]);
  }

  // Handle Enter key in form fields
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      if (!isConnected) {
        testConnection();
      } else if (selectedDatabase) {
        connectToDatabase();
      }
    }
  }

  // Auto-connect when a database is selected
  $: if (
    selectedDatabase &&
    selectedDatabase !== "" &&
    selectedDatabase !== "Choose a database..." &&
    isConnected
  ) {
    connectToDatabase();
  }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block">
    <!-- Connection Form -->
    <div class="connection-section">
      <span class="header">
        <h2>Server Login</h2>
        {#if isConnected}
          <button class="cta-button cancel" on:click={resetConnection}>
            Reset Connection
          </button>
        {:else}
          <button
            class="cta-button"
            on:click={testConnection}
            disabled={isConnecting ||
              isConnected ||
              !username ||
              !password ||
              !address ||
              !port}
          >
            {#if isConnecting}
              Connecting...
            {:else}
              Connect
            {/if}
          </button>
        {/if}
      </span>

      <div class="form-group">
        <label for="username">Username:</label>
        <input
          class="input-field"
          style="margin-left: 0px"
          id="username"
          type="text"
          bind:value={username}
          placeholder="Enter username"
          disabled={isConnecting || isConnected}
          on:keydown={handleKeydown}
        />
      </div>

      <div class="form-group">
        <label for="password">Password:</label>
        <input
          class="input-field"
          style="margin-left: 0px"
          id="password"
          type="password"
          bind:value={password}
          placeholder="Enter password"
          disabled={isConnecting || isConnected}
          on:keydown={handleKeydown}
        />
      </div>

      <div class="form-group">
        <label for="address">Address:</label>
        <input
          class="input-field"
          style="margin-left: 0px"
          id="address"
          type="text"
          bind:value={address}
          placeholder="e.g., localhost or server.company.com"
          disabled={isConnecting || isConnected}
          on:keydown={handleKeydown}
        />
      </div>

      <div class="form-group">
        <label for="port">Port:</label>
        <input
          class="input-field"
          style="margin-left: 0px"
          id="port"
          type="number"
          bind:value={port}
          placeholder="3306"
          min="1"
          max="65535"
          disabled={isConnecting || isConnected}
          on:keydown={handleKeydown}
        />
      </div>

      {#if isConnected && availableDatabases.length > 0}
        <span class="header">
          <div
            class="input-group"
            style="margin-top: 20px; align-items: center; flex: 1"
          >
            <select
              class="select-field"
              style="margin-left: 0px"
              id="database"
              bind:value={selectedDatabase}
              disabled={isDatabasesLoading}
            >
              <option value="">Choose a database...</option>
              {#each availableDatabases as db}
                <option value={db}>{db}</option>
              {/each}
            </select>
          </div>
          <!-- 
          <button
            class="cta-button"
            style="margin-left: 20px; margin-top: 10px"
            on:click={connectToDatabase}
            disabled={!selectedDatabase || isDatabasesLoading}
          >
            Open Database
          </button> -->
        </span>
      {/if}
      {#if connectionError}
        <div class="error-message">
          {connectionError}
        </div>
      {/if}

      <!-- Success State -->
      <!-- {#if isConnected && selectedDatabase}
        <div class="success-message">
          Successfully connected to {selectedDatabase} on {address}:{port}
        </div>
      {/if} -->
    </div>

    <!-- Error Display -->
  </div>
</div>

<style>
  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    color: var(--text-color);
    font-weight: 500;
    font-size: 14px;
  }

  .form-group input {
    width: 100%;
    padding: 10px 12px;
    background-color: var(--primary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 4px;
    color: var(--text-color);
    font-size: 14px;
    transition: border-color 0.2s ease;
    box-sizing: border-box;
  }

  .form-group input:focus {
    outline: none;
    border-color: var(--accent-color);
    box-shadow: 0 0 0 2px rgba(240, 165, 0, 0.2);
  }

  .form-group input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    color: var(--inactive-color);
  }

  .form-group input::placeholder {
    color: var(--inactive-color);
  }

  /* Browser-specific placeholder styling for better compatibility */
  .form-group input::-webkit-input-placeholder {
    color: var(--inactive-color);
  }

  .form-group input::-moz-placeholder {
    color: var(--inactive-color);
    opacity: 1;
  }

  .form-group input:-ms-input-placeholder {
    color: var(--inactive-color);
  }

  .error-message {
    background-color: var(--warning-color);
    color: var(--text-color);
    padding: 12px;
    border-radius: 4px;
    margin-top: 15px;
    font-size: 14px;
    text-align: center;
  }
</style>
