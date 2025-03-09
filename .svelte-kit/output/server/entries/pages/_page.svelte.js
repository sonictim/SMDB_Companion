import { Y as current_component, S as push, Z as store_get, _ as ensure_array_like, $ as stringify, a0 as await_block, a1 as unsubscribe_stores, a2 as bind_props, V as pop, a3 as fallback, a4 as copy_payload, a5 as assign_payload } from "../../chunks/index.js";
import { Search, CheckSquare, Square, OctagonX, NotebookPenIcon, TriangleAlert, Database, FilesIcon, Settings2 } from "lucide-svelte";
import { a as attr, t as to_class, c as clsx, b as algorithmsStore, p as preferencesStore, r as registrationStore } from "../../chunks/Select.svelte_svelte_type_style_lang.js";
import "@tauri-apps/api/core";
import "@tauri-apps/api/window";
import "@tauri-apps/api/webviewWindow";
import "@tauri-apps/api/menu";
import "@tauri-apps/api/event";
import { w as writable, g as get } from "../../chunks/index2.js";
import "@tauri-apps/plugin-dialog";
import { basename, extname } from "@tauri-apps/api/path";
import { e as escape_html } from "../../chunks/escaping.js";
function onDestroy(fn) {
  var context = (
    /** @type {Component} */
    current_component
  );
  (context.d ??= []).push(fn);
}
const metadataDefault = { find: "", replace: "", column: "FilePath", case_sensitive: false, mark_dirty: true };
const resultsStore = writable(
  JSON.parse(sessionStorage.getItem("results") || "[]")
);
let initialMetadata;
try {
  const storedMetadata = sessionStorage.getItem("metadata");
  initialMetadata = storedMetadata ? JSON.parse(storedMetadata) : metadataDefault;
} catch (e) {
  console.error("Error loading metadata:", e);
  initialMetadata = metadataDefault;
}
const metadataStore = writable(initialMetadata);
resultsStore.subscribe((value) => {
  sessionStorage.setItem("results", JSON.stringify(value));
});
metadataStore.subscribe((value) => {
  sessionStorage.setItem("metadata", JSON.stringify(value));
});
function Search_1($$payload, $$props) {
  push();
  var $$store_subs;
  let metadata, isBasicEnabled;
  let dbSize = $$props["dbSize"];
  let activeTab = $$props["activeTab"];
  let isRemove = $$props["isRemove"];
  let selectedDb = $$props["selectedDb"];
  async function getFilenameWithoutExtension(fullPath) {
    const name = await basename(fullPath);
    const ext = await extname(fullPath);
    return name.replace(ext, "");
  }
  let pref = get(preferencesStore);
  get(algorithmsStore);
  function getAlgoClass(algo, algorithms) {
    if ((algo.id === "audiosuite" || algo.id === "filename") && // Add filename check here
    !algorithms.find((a) => a.id === "basic")?.enabled) {
      return "inactive";
    }
    return "";
  }
  onDestroy(() => {
  });
  metadata = metadataStore;
  isBasicEnabled = store_get($$store_subs ??= {}, "$algorithmsStore", algorithmsStore).find((a) => a.id === "basic")?.enabled || false;
  const each_array_1 = ensure_array_like(pref.columns);
  $$payload.out += `<div class="page-columns svelte-1xywobe"><div class="block" style="height: 40vh"><div class="header"><h2>Search Algorithms</h2> `;
  if (selectedDb == null) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<button class="cta-button inactive">`;
    Search($$payload, { size: 18 });
    $$payload.out += `<!----> <span>Search</span></button>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<button${attr("class", `cta-button ${stringify("")}`)}><div class="flex items-center gap-2">`;
    {
      $$payload.out += "<!--[!-->";
      Search($$payload, { size: 18 });
      $$payload.out += `<!----> <span>Search</span>`;
    }
    $$payload.out += `<!--]--></div></button>`;
  }
  $$payload.out += `<!--]--></div> `;
  {
    $$payload.out += "<!--[!-->";
    const each_array = ensure_array_like(store_get($$store_subs ??= {}, "$algorithmsStore", algorithmsStore));
    $$payload.out += `<div class="grid"><!--[-->`;
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let algo = each_array[$$index];
      $$payload.out += `<div${attr("class", to_class("grid item " + getAlgoClass(algo, store_get($$store_subs ??= {}, "$algorithmsStore", algorithmsStore)), "svelte-1xywobe"))}><button type="button" class="grid item">`;
      if (algo.id === "audiosuite" || algo.id === "filename") {
        $$payload.out += "<!--[-->";
        $$payload.out += `<span style="margin-right: 20px;"></span>`;
      } else {
        $$payload.out += "<!--[!-->";
      }
      $$payload.out += `<!--]--> `;
      if (algo.enabled) {
        $$payload.out += "<!--[-->";
        CheckSquare($$payload, {
          size: 20,
          class: `checkbox ${stringify((algo.id === "audiosuite" || algo.id === "filename") && !isBasicEnabled ? "inactive" : "checked")}`
        });
      } else {
        $$payload.out += "<!--[!-->";
        Square($$payload, { size: 20, class: "checkbox inactive" });
      }
      $$payload.out += `<!--]--> <span${attr("class", clsx((algo.id === "audiosuite" || algo.id === "filename") && !isBasicEnabled ? "inactive" : ""))}>${escape_html(algo.name)}</span></button> `;
      if (algo.id === "dbcompare") {
        $$payload.out += "<!--[-->";
        if (algo.db !== null && algo.db !== void 0) {
          $$payload.out += "<!--[-->";
          $$payload.out += `<!---->`;
          await_block(
            getFilenameWithoutExtension(algo.db),
            () => {
            },
            (filename) => {
              $$payload.out += `<span class="clickable">${escape_html(filename)}</span>`;
            }
          );
          $$payload.out += `<!---->`;
        } else {
          $$payload.out += "<!--[!-->";
          $$payload.out += `<button type="button" class="small-button">Select SQLite File</button>`;
        }
        $$payload.out += `<!--]-->`;
      } else {
        $$payload.out += "<!--[!-->";
      }
      $$payload.out += `<!--]--> `;
      if (algo.id === "duration") {
        $$payload.out += "<!--[-->";
        $$payload.out += `<input type="number" min="0" step="0.1"${attr("value", algo.min_dur)} class="duration-input" style="width: 55px"> s`;
      } else {
        $$payload.out += "<!--[!-->";
      }
      $$payload.out += `<!--]--></div>`;
    }
    $$payload.out += `<!--]--></div>`;
  }
  $$payload.out += `<!--]--></div> <div class="block" style="height: 100%; margin-top: 20px"><div class="header"><h2>Metadata Replacement</h2> `;
  if (selectedDb == null) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<button class="cta-button inactive" style="width: 125px">`;
    Search($$payload, { size: 18 });
    $$payload.out += `<!----> <span>Find</span></button>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<button class="cta-button" style="width: 125px">`;
    Search($$payload, { size: 18 });
    $$payload.out += `<!----> <span>Find</span></button>`;
  }
  $$payload.out += `<!--]--></div> <div class="input-group2"><label for="case-sensitive"><button type="button" class="grid item">`;
  if (store_get($$store_subs ??= {}, "$metadata", metadata).case_sensitive) {
    $$payload.out += "<!--[-->";
    CheckSquare($$payload, { size: 20, class: "checkbox checked" });
  } else {
    $$payload.out += "<!--[!-->";
    Square($$payload, { size: 20, class: "checkbox" });
  }
  $$payload.out += `<!--]--> <span>Case Sensitive</span></button></label></div> <div class="input-group"><label for="find-text">Find:</label> <input type="text" id="find-text"${attr("value", store_get($$store_subs ??= {}, "$metadata", metadata).find)} placeholder="Enter text to find" class="input-field"></div> <div class="input-group"><label for="replace-text">Replace:</label> <input type="text" id="replace-text"${attr("value", store_get($$store_subs ??= {}, "$metadata", metadata).replace)} placeholder="Enter text to replace" class="input-field"></div> <div class="input-group"><label for="column-select">in Column:</label> <select id="column-select" class="select-field"><!--[-->`;
  for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
    let option = each_array_1[$$index_1];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select></div></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  bind_props($$props, { dbSize, activeTab, isRemove, selectedDb });
  pop();
}
function Results($$payload, $$props) {
  push();
  var $$store_subs;
  let pref, metadata;
  let removeResults = $$props["removeResults"];
  let isRemove = $$props["isRemove"];
  let activeTab = $$props["activeTab"];
  let selectedDb = fallback($$props["selectedDb"], null);
  let items = [];
  let total = 0;
  let selectedItems = /* @__PURE__ */ new Set();
  let currentFilter = "Relevant";
  function filterItems(items2, filter) {
    switch (filter) {
      case "All":
        return items2;
      case "Relevant":
        return items2.filter((item) => !item.algorithm.includes("Keep") || item.algorithm.length > 1);
      case "Keep":
        return items2.filter((item) => item.algorithm.includes("Keep"));
      case "Remove":
        return items2.filter((item) => !item.algorithm.includes("Keep"));
      default:
        return items2.filter((item) => item.algorithm.includes(filter));
    }
  }
  let columnConfigs = [
    { minWidth: 10, width: 20, percentage: 2 },
    // Checkbox
    { minWidth: 100, width: 200, percentage: 30 },
    // Root
    { minWidth: 150, width: 300, percentage: 53 },
    // Path
    { minWidth: 50, width: 50, percentage: 15 }
    // Algorithm
  ];
  let filters = [
    { id: "All", name: "All Records" },
    { id: "Relevant", name: "Relevant Records" },
    { id: "Keep", name: "Records to Keep" },
    { id: "Remove", name: "Records to Remove" },
    { id: "Basic", name: "Basic Duplicate Search" },
    { id: "InvalidPath", name: "Valid Filename" },
    {
      id: "SimilarFilename",
      name: "Similar Filename Search"
    },
    { id: "Tags", name: "Audiosuite Tags" },
    { id: "Waveform", name: "Waveform Comparison" },
    { id: "Duration", name: "Duration" },
    { id: "Compare", name: "Database Compare" }
  ];
  pref = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  store_get($$store_subs ??= {}, "$resultsStore", resultsStore);
  metadata = store_get($$store_subs ??= {}, "$metadataStore", metadataStore);
  {
    let newFiltered = filterItems(items, currentFilter);
    newFiltered.forEach((item) => {
      if (selectedItems.has(item.id)) {
        selectedItems.add(item.id);
      }
    });
  }
  columnConfigs.map((config) => config.width);
  const each_array_1 = ensure_array_like([
    { bool: true, text: "Database Copy with tag:" },
    { bool: false, text: "Current Database" }
  ]);
  const each_array_2 = ensure_array_like([
    { id: "Keep", text: "Keep" },
    { id: "Trash", text: "Move To Trash" },
    { id: "Delete", text: "Permanently Delete" }
  ]);
  $$payload.out += `<div class="block svelte-cyleiw"><div class="header svelte-cyleiw"><h2 class="svelte-cyleiw">Search Results:</h2> <span style="font-size: 18px" class="svelte-cyleiw">`;
  if (isRemove) {
    $$payload.out += "<!--[-->";
    $$payload.out += `${escape_html(total)} of ${escape_html(items.length)} Records marked for Removal`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `${escape_html(items.length)} Records found`;
  }
  $$payload.out += `<!--]--></span> <div style="margin-left: auto; display: flex; gap: 20px;" class="svelte-cyleiw">`;
  if (isRemove) {
    $$payload.out += "<!--[-->";
    {
      $$payload.out += "<!--[!-->";
    }
    $$payload.out += `<!--]--> <button class="cta-button cancel svelte-cyleiw">`;
    OctagonX($$payload, { size: "18" });
    $$payload.out += `<!----> Remove Checked</button>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<button class="cta-button cancel svelte-cyleiw">`;
    NotebookPenIcon($$payload, { size: "18" });
    $$payload.out += `<!----> <span class="svelte-cyleiw">Replace '${escape_html(metadata.find)}' with '${escape_html(metadata?.replace || "")}'</span></button>`;
  }
  $$payload.out += `<!--]--></div></div> <div class="bar svelte-cyleiw" style="margin-bottom: 16px; margin-top: 10px"><button type="button" class="grid item svelte-cyleiw">`;
  {
    $$payload.out += "<!--[!-->";
    Square($$payload, { size: 20, class: "checkbox" });
  }
  $$payload.out += `<!--]--> <span class="svelte-cyleiw">Enable Selections</span></button> `;
  {
    $$payload.out += "<!--[!-->";
  }
  $$payload.out += `<!--]--> <div class="filter-container svelte-cyleiw">`;
  if (isRemove) {
    $$payload.out += "<!--[-->";
    const each_array = ensure_array_like(filters);
    $$payload.out += `<span class="svelte-cyleiw">Filter by:</span> <select class="select-field svelte-cyleiw"><!--[-->`;
    for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
      let option = each_array[$$index];
      $$payload.out += `<option${attr("value", option.id)} class="svelte-cyleiw">${escape_html(option.name)}</option>`;
    }
    $$payload.out += `<!--]--></select>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<button type="button" class="grid item svelte-cyleiw" style="margin-left: 120px">`;
    if (store_get($$store_subs ??= {}, "$metadataStore", metadataStore).mark_dirty) {
      $$payload.out += "<!--[-->";
      CheckSquare($$payload, {
        size: 20,
        class: `checkbox checked ${stringify(metadata.column == "FilePath" || metadata.column == "Filename" || metadata.column == "Pathname" ? "inactive" : "")}`
      });
    } else {
      $$payload.out += "<!--[!-->";
      Square($$payload, { size: 20, class: "checkbox" });
    }
    $$payload.out += `<!--]--> <span${attr("class", to_class(clsx(metadata.column == "FilePath" || metadata.column == "Filename" || metadata.column == "Pathname" ? "inactive" : ""), "svelte-cyleiw"))}>Mark Records as Dirty</span></button>`;
  }
  $$payload.out += `<!--]--></div></div> <div class="block inner svelte-cyleiw" style="margin-bottom: 15px">`;
  {
    $$payload.out += "<!--[-->";
    $$payload.out += `<p class="ellipsis svelte-cyleiw">Loading data...</p>`;
  }
  $$payload.out += `<!--]--></div> <div class="header svelte-cyleiw" style="margin-bottom: 0px"><span${attr("style", pref.safety_db ? "" : "color: var(--warning-hover); font-style: bold;")} class="svelte-cyleiw">Remove Records From: <select class="select-field svelte-cyleiw"><!--[-->`;
  for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
    let option = each_array_1[$$index_1];
    $$payload.out += `<option${attr("value", option.bool)} class="svelte-cyleiw">${escape_html(option.text)}</option>`;
  }
  $$payload.out += `<!--]--></select> `;
  if (pref.safety_db) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<input class="input-field svelte-cyleiw" placeholder="_thinned" type="text" id="new_db_tag"${attr("value", pref.safety_db_tag)}>`;
  } else {
    $$payload.out += "<!--[!-->";
    TriangleAlert($$payload, { size: "20", class: "blinking" });
  }
  $$payload.out += `<!--]--></span> <span${attr("style", pref.erase_files != "Keep" ? "color: var(--warning-hover); font-style: bold;" : "")} class="svelte-cyleiw">`;
  if (pref.erase_files === "Delete") {
    $$payload.out += "<!--[-->";
    TriangleAlert($$payload, { size: "20", class: "blinking" });
  } else {
    $$payload.out += "<!--[!-->";
  }
  $$payload.out += `<!--]--> Duplicate Files On Disk: <select class="select-field svelte-cyleiw"><!--[-->`;
  for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
    let option = each_array_2[$$index_2];
    $$payload.out += `<option${attr("value", option.id)} class="svelte-cyleiw">${escape_html(option.text)}</option>`;
  }
  $$payload.out += `<!--]--></select></span></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  bind_props($$props, {
    removeResults,
    isRemove,
    activeTab,
    selectedDb
  });
  pop();
}
function Metadata($$payload, $$props) {
  push();
  let activeTab = $$props["activeTab"];
  let isRemove = $$props["isRemove"];
  let selectedDb = $$props["selectedDb"];
  let pref = get(preferencesStore);
  let findText = "";
  let replaceText = "";
  const each_array = ensure_array_like(pref.columns);
  $$payload.out += `<div class="block"><div class="header"><h2>Metadata Replacement</h2> `;
  if (selectedDb == null) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<button class="cta-button inactive">`;
    Search($$payload, { size: 18 });
    $$payload.out += `<!----> <span>Find Records</span></button>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<button class="cta-button">`;
    Search($$payload, { size: 18 });
    $$payload.out += `<!----> <span>Find Records</span></button>`;
  }
  $$payload.out += `<!--]--></div> <div class="input-group2"><label for="case-sensitive"><button type="button" class="grid item">`;
  {
    $$payload.out += "<!--[!-->";
    Square($$payload, { size: 20, class: "checkbox" });
  }
  $$payload.out += `<!--]--> <span>Case Sensitive</span></button></label></div> <div class="input-group"><label for="find-text">Find:</label> <input type="text" id="find-text"${attr("value", findText)} placeholder="Enter text to find" class="input-field"></div> <div class="input-group"><label for="replace-text">Replace:</label> <input type="text" id="replace-text"${attr("value", replaceText)} placeholder="Enter text to replace" class="input-field"></div> <div class="input-group"><label for="column-select">in Column:</label> <select id="column-select" class="select-field"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let option = each_array[$$index];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select></div></div>`;
  bind_props($$props, { activeTab, isRemove, selectedDb });
  pop();
}
function Register($$payload, $$props) {
  push();
  let isRegistered = $$props["isRegistered"];
  let reg = get(registrationStore);
  let total = 0;
  let r2 = "temp";
  $$payload.out += `<div class="block"><div class="header"><h2>Search Results: <span class="basic-text" style="display: inline-flex; margin: 0;">${escape_html(total)} duplicates found</span></h2> <button class="cta-button cancel">Register</button></div> <div class="input-group2"><label for="case-sensitive">Registration Required to View Results</label></div> <div class="input-group"><label for="name">Name:</label> <input type="text" id="find-text"${attr("value", reg.name)} placeholder="Enter Registration Name" class="input-field"></div> <div class="input-group"><label for="email">Email:</label> <input type="text" id="find-text"${attr("value", reg.email)} placeholder="Enter Registration Email" class="input-field"></div> <div class="input-group"><label for="Reg">License:</label> <input type="text" id="replace-text"${attr("value", reg.license)} placeholder="Enter License number" class="input-field"></div> ${escape_html(r2)}</div>`;
  bind_props($$props, { isRegistered });
  pop();
}
function _page($$payload, $$props) {
  push();
  let dbSize = fallback($$props["dbSize"], 0);
  let activeTab = fallback($$props["activeTab"], "search");
  let isRemove = fallback($$props["isRemove"], true);
  let isRegistered = fallback($$props["isRegistered"], false);
  let selectedDb = fallback($$props["selectedDb"], null);
  get(preferencesStore);
  preferencesStore.subscribe((value) => {
    if (value?.colors) {
      const { colors } = value;
      document.documentElement.style.setProperty("--primary-bg", colors.primaryBg ?? "#1a1a1a");
      document.documentElement.style.setProperty("--secondary-bg", colors.secondaryBg ?? "#2a2a2a");
      document.documentElement.style.setProperty("--text-color", colors.textColor ?? "#ffffff");
      document.documentElement.style.setProperty("--topbar-color", colors.topbarColor ?? "#333333");
      document.documentElement.style.setProperty("--accent-color", colors.accentColor ?? "#007acc");
      document.documentElement.style.setProperty("--hover-color", colors.hoverColor ?? "#2b2b2b");
      document.documentElement.style.setProperty("--warning-color", colors.warningColor ?? "#ff4444");
      document.documentElement.style.setProperty("--warning-hover", colors.warningHover ?? "#cc0000");
      document.documentElement.style.setProperty("--inactive-color", colors.inactiveColor ?? "#666666");
    }
  });
  function removeResults() {
    console.log("Remove selected results");
  }
  let $$settled = true;
  let $$inner_payload;
  function $$render_inner($$payload2) {
    $$payload2.out += `<div class="app-container"><div class="top-bar"><div class="top-bar-left"><button class="nav-link">`;
    Database($$payload2, { size: 18 });
    $$payload2.out += `<!----> <span style="font-size: 24px;">${escape_html(selectedDb ? selectedDb : "Select Database")}</span></button></div> <div class="top-bar-right"><button${attr("class", `nav-link ${stringify(activeTab === "search" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
    Search($$payload2, { size: 18 });
    $$payload2.out += `<!----> <span>Search</span></div></button> <button${attr("class", `nav-link ${stringify(activeTab === "results" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
    FilesIcon($$payload2, { size: 18 });
    $$payload2.out += `<!----> <span>Results</span></div></button> <button class="nav-link"><div class="flex items-center gap-2">`;
    Settings2($$payload2, { size: 18 });
    $$payload2.out += `<!----> Options</div></button></div></div> <main class="content">`;
    if (activeTab === "search") {
      $$payload2.out += "<!--[-->";
      Search_1($$payload2, {
        dbSize,
        get selectedDb() {
          return selectedDb;
        },
        set selectedDb($$value) {
          selectedDb = $$value;
          $$settled = false;
        },
        get activeTab() {
          return activeTab;
        },
        set activeTab($$value) {
          activeTab = $$value;
          $$settled = false;
        },
        get isRemove() {
          return isRemove;
        },
        set isRemove($$value) {
          isRemove = $$value;
          $$settled = false;
        }
      });
    } else {
      $$payload2.out += "<!--[!-->";
      if (activeTab === "results") {
        $$payload2.out += "<!--[-->";
        if (isRegistered) {
          $$payload2.out += "<!--[-->";
          Results($$payload2, {
            removeResults,
            get isRemove() {
              return isRemove;
            },
            set isRemove($$value) {
              isRemove = $$value;
              $$settled = false;
            },
            get activeTab() {
              return activeTab;
            },
            set activeTab($$value) {
              activeTab = $$value;
              $$settled = false;
            },
            get selectedDb() {
              return selectedDb;
            },
            set selectedDb($$value) {
              selectedDb = $$value;
              $$settled = false;
            }
          });
        } else {
          $$payload2.out += "<!--[!-->";
          Register($$payload2, {
            get isRegistered() {
              return isRegistered;
            },
            set isRegistered($$value) {
              isRegistered = $$value;
              $$settled = false;
            }
          });
        }
        $$payload2.out += `<!--]-->`;
      } else {
        $$payload2.out += "<!--[!-->";
        if (activeTab === "metadata") {
          $$payload2.out += "<!--[-->";
          Metadata($$payload2, {
            get activeTab() {
              return activeTab;
            },
            set activeTab($$value) {
              activeTab = $$value;
              $$settled = false;
            },
            get isRemove() {
              return isRemove;
            },
            set isRemove($$value) {
              isRemove = $$value;
              $$settled = false;
            },
            get selectedDb() {
              return selectedDb;
            },
            set selectedDb($$value) {
              selectedDb = $$value;
              $$settled = false;
            }
          });
        } else {
          $$payload2.out += "<!--[!-->";
        }
        $$payload2.out += `<!--]-->`;
      }
      $$payload2.out += `<!--]-->`;
    }
    $$payload2.out += `<!--]--></main></div>`;
  }
  do {
    $$settled = true;
    $$inner_payload = copy_payload($$payload);
    $$render_inner($$inner_payload);
  } while (!$$settled);
  assign_payload($$payload, $$inner_payload);
  bind_props($$props, {
    dbSize,
    activeTab,
    isRemove,
    isRegistered,
    selectedDb
  });
  pop();
}
export {
  _page as default
};
