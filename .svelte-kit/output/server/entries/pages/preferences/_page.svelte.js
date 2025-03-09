import { a3 as fallback, _ as ensure_array_like, a6 as slot, a2 as bind_props, V as pop, S as push, $ as stringify, Z as store_get, a1 as unsubscribe_stores, a7 as invalid_default_snippet } from "../../../chunks/index.js";
import { OctagonX, ArrowBigRight, ArrowBigLeft, CheckSquare, Square, GripVertical, ListCheck, ListOrdered, Tags as Tags$1, Palette } from "lucide-svelte";
import { a as attr, p as preferencesStore, t as to_class, P as PresetsStore } from "../../../chunks/Select.svelte_svelte_type_style_lang.js";
import "@tauri-apps/api/core";
import { e as escape_html } from "../../../chunks/escaping.js";
import "@tauri-apps/api/event";
function VirtualList($$payload, $$props) {
  push();
  let items = $$props["items"];
  let height = fallback($$props["height"], "100%");
  let itemHeight = fallback($$props["itemHeight"], void 0);
  let start = fallback($$props["start"], 0);
  let end = fallback($$props["end"], 0);
  let visible;
  let top = 0;
  let bottom = 0;
  visible = items.slice(start, end).map((data, i) => {
    return { index: i + start, data };
  });
  const each_array = ensure_array_like(visible);
  $$payload.out += `<svelte-virtual-list-viewport${attr("style", `height: ${stringify(height)};`)} class="svelte-1tqh76q"><svelte-virtual-list-contents${attr("style", `padding-top: ${stringify(top)}px; padding-bottom: ${stringify(bottom)}px;`)} class="svelte-1tqh76q"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let row = each_array[$$index];
    $$payload.out += `<svelte-virtual-list-row class="svelte-1tqh76q"><!---->`;
    slot($$payload, $$props, "default", { item: row.data }, () => {
      $$payload.out += `Missing template`;
    });
    $$payload.out += `<!----></svelte-virtual-list-row>`;
  }
  $$payload.out += `<!--]--></svelte-virtual-list-contents></svelte-virtual-list-viewport>`;
  bind_props($$props, { items, height, itemHeight, start, end });
  pop();
}
function Select($$payload, $$props) {
  push();
  var $$store_subs;
  let pref;
  let newSelect;
  let newTag;
  let selectedItems = /* @__PURE__ */ new Set();
  let selectedTags = /* @__PURE__ */ new Set();
  pref = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  $$payload.out += `<div class="page-columns svelte-l0wmiu"><div><div class="block"><div class="header"><h2>Audiosuite Tags</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: "18" });
  $$payload.out += `<!----> Remove</button></div> <div class="bar"><button class="cta-button small">Add</button> <input type="text" id="find-text"${attr("value", newTag)} placeholder="New Tag" class="input-field"></div> <div class="block inner">`;
  VirtualList($$payload, {
    items: Array.from(pref.tags),
    children: invalid_default_snippet,
    $$slots: {
      default: ($$payload2, { item }) => {
        $$payload2.out += `<div${attr("class", to_class("list-item", null, {
          "selected-item": selectedTags.has(item),
          "unselected-item": !selectedTags.has(item)
        }))}>${escape_html(item)}</div>`;
      }
    }
  });
  $$payload.out += `<!----></div></div></div> <div class="arrow-column svelte-l0wmiu"><div class="move-button-container svelte-l0wmiu"><button class="arrow-button svelte-l0wmiu">`;
  ArrowBigRight($$payload, { size: "100" });
  $$payload.out += `<!----></button> <button class="arrow-button svelte-l0wmiu">`;
  ArrowBigLeft($$payload, { size: "100" });
  $$payload.out += `<!----></button></div></div> <div><div class="block"><div class="header"><h2>Filename Tags</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: "18" });
  $$payload.out += `<!----> Remove</button></div> <div class="bar"><button class="cta-button small">Add</button> <input type="text" id="find-text"${attr("value", newSelect)} placeholder="Add New String" class="input-field"></div> <div class="block inner">`;
  VirtualList($$payload, {
    items: Array.from(pref.autoselects),
    children: invalid_default_snippet,
    $$slots: {
      default: ($$payload2, { item }) => {
        $$payload2.out += `<div${attr("class", to_class("list-item", null, {
          "selected-item": selectedItems.has(item),
          "unselected-item": !selectedItems.has(item)
        }))}>${escape_html(item)}</div>`;
      }
    }
  });
  $$payload.out += `<!----></div></div></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function Main($$payload, $$props) {
  push();
  var $$store_subs;
  let preset = ["default", "TJF"];
  store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  const each_array = ensure_array_like(["New Database", "Current Database"]);
  const each_array_1 = ensure_array_like([
    "Keep",
    "Move To Trash",
    "Permanently Delete"
  ]);
  const each_array_2 = ensure_array_like(preset);
  $$payload.out += `<div class="block"><div class="header"><h2>Configuration Options</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: 18 });
  $$payload.out += `<!----> Reset Preferences</button></div> <div class="block inner"><div class="grid"><span>Remove Records From: <select class="select-field"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let option = each_array[$$index];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select></span> <span>Duplicate Files On Disk: <select class="select-field"><!--[-->`;
  for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
    let option = each_array_1[$$index_1];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select></span> <span>New Database Tag: <input class="input-field" placeholder="-thinned"></span> <button type="button" class="grid item">`;
  if (store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore).ignore_filetype) {
    $$payload.out += "<!--[-->";
    CheckSquare($$payload, { size: 20, class: "checkbox checked" });
  } else {
    $$payload.out += "<!--[!-->";
    Square($$payload, { size: 20, class: "checkbox" });
  }
  $$payload.out += `<!--]--> <span>Ignore Filetypes (extensions)</span></button></div></div> <div class="bar" style="margin-top: 20px;"><button class="cta-button small">Save:</button> <input type="text" class="input-field" placeholder="Enter New Configuraion Preset Name" style="margin-right: 20px;"> <button class="cta-button small">Load:</button> <select class="select-field" style="margin-right: 10px;"><!--[-->`;
  for (let $$index_2 = 0, $$length = each_array_2.length; $$index_2 < $$length; $$index_2++) {
    let p = each_array_2[$$index_2];
    $$payload.out += `<option${attr("value", p)}>${escape_html(p)}</option>`;
  }
  $$payload.out += `<!--]--></select> <button class="cta-button small cancel">Delete</button></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function Match($$payload, $$props) {
  push();
  var $$store_subs;
  let filteredColumns;
  let selectedMatches = /* @__PURE__ */ new Set();
  store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  filteredColumns = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore).columns.filter((col) => !store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore).match_criteria.includes(col));
  const each_array = ensure_array_like(filteredColumns);
  $$payload.out += `<div class="block svelte-wm3ozt"><div class="header"><h2>Duplicate Match Criteria</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: "18" });
  $$payload.out += `<!----> Remove Selected</button></div> <div class="bar"><div class="button-group"><span>Add:</span> <select class="select-field"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let option = each_array[$$index];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select></div> <button type="button" class="grid item" style="margin-left: 120px">`;
  if (store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore).ignore_filetype) {
    $$payload.out += "<!--[-->";
    CheckSquare($$payload, { size: 20, class: "checkbox checked" });
  } else {
    $$payload.out += "<!--[!-->";
    Square($$payload, { size: 20, class: "checkbox" });
  }
  $$payload.out += `<!--]--> <span>Ignore Filetypes (extensions)</span></button></div> <div class="block inner svelte-wm3ozt">`;
  VirtualList($$payload, {
    items: Array.from(store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore).match_criteria),
    children: invalid_default_snippet,
    $$slots: {
      default: ($$payload2, { item }) => {
        $$payload2.out += `<div${attr("class", to_class("list-item", null, {
          "selected-item": selectedMatches.has(item),
          "unselected-item": !selectedMatches.has(item)
        }))}>${escape_html(item)}</div>`;
      }
    }
  });
  $$payload.out += `<!----></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function Order($$payload, $$props) {
  push();
  var $$store_subs;
  let pref;
  let selectedItems = /* @__PURE__ */ new Set();
  let newOption = "";
  let currentOperator = "Contains";
  let dragSourceIndex = null;
  let dropTargetIndex = null;
  let operators = [
    { id: "Is", name: "is" },
    { id: "IsNot", name: "is NOT" },
    { id: "Largest", name: "is largest" },
    { id: "Smallest", name: "is smallest" },
    { id: "IsEmpty", name: "is empty" },
    { id: "IsNotEmpty", name: "is NOT empty" },
    { id: "Contains", name: "contains" },
    {
      id: "DoesNotContain",
      name: "does NOT contain"
    }
  ];
  pref = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  const each_array = ensure_array_like(pref.columns);
  const each_array_1 = ensure_array_like(operators);
  const each_array_2 = ensure_array_like(pref.preservation_order);
  $$payload.out += `<div class="block svelte-s9hvzz"><div class="header"><h2>Duplicate Selection Logic</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: 18 });
  $$payload.out += `<!----> Remove Selected</button></div> <div class="bar"><button class="cta-button small">Add</button> <select class="select-field" style="flex-grow: 0"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let option = each_array[$$index];
    $$payload.out += `<option${attr("value", option)}>${escape_html(option)}</option>`;
  }
  $$payload.out += `<!--]--></select> <select class="select-field" style="flex-grow: 0"><!--[-->`;
  for (let $$index_1 = 0, $$length = each_array_1.length; $$index_1 < $$length; $$index_1++) {
    let option = each_array_1[$$index_1];
    $$payload.out += `<option${attr("value", option.id)}>${escape_html(option.name)}</option>`;
  }
  $$payload.out += `<!--]--></select> `;
  if (["Contains", "DoesNotContain", "Is", "IsNot"].includes(currentOperator)) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<input type="text"${attr("value", newOption)} placeholder="Enter New Option" class="input-field">`;
  } else {
    $$payload.out += "<!--[!-->";
  }
  $$payload.out += `<!--]--></div> <div class="block inner svelte-s9hvzz"><!--[-->`;
  for (let i = 0, $$length = each_array_2.length; i < $$length; i++) {
    let item = each_array_2[i];
    $$payload.out += `<div${attr("class", to_class("item-container", "svelte-s9hvzz", { "drop-target": dropTargetIndex === i }))}><div${attr("class", to_class("list-item", "svelte-s9hvzz", {
      "selected-item": selectedItems.has(item),
      "unselected-item": !selectedItems.has(item)
    }))} draggable="true" role="listitem"><div class="drag-handle svelte-s9hvzz">`;
    GripVertical($$payload, { size: 18 });
    $$payload.out += `<!----></div> <div class="item-content svelte-s9hvzz">${escape_html(i + 1)}. ${escape_html(item.column)} `;
    if (operators.some((op) => op.id === item.operator)) {
      $$payload.out += "<!--[-->";
      $$payload.out += `${escape_html(operators.find((op) => op.id === item.operator)?.name)}`;
    } else {
      $$payload.out += "<!--[!-->";
    }
    $$payload.out += `<!--]--> `;
    if (["Contains", "DoesNotContain", "Is", "IsNot"].includes(item.operator)) {
      $$payload.out += "<!--[-->";
      $$payload.out += `\`${escape_html(item.variable)}\``;
    } else {
      $$payload.out += "<!--[!-->";
    }
    $$payload.out += `<!--]--></div></div> `;
    if (dropTargetIndex === i && dragSourceIndex !== i) {
      $$payload.out += "<!--[-->";
      $$payload.out += `<div class="drop-indicator svelte-s9hvzz"></div>`;
    } else {
      $$payload.out += "<!--[!-->";
    }
    $$payload.out += `<!--]--></div>`;
  }
  $$payload.out += `<!--]-->  <div${attr("class", to_class("end-drop-area", "svelte-s9hvzz", {
    "drop-target": dropTargetIndex === pref.preservation_order.length
  }))}>`;
  if (dropTargetIndex === pref.preservation_order.length) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<div class="drop-indicator svelte-s9hvzz"></div>`;
  } else {
    $$payload.out += "<!--[!-->";
  }
  $$payload.out += `<!--]--></div></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function Tags($$payload, $$props) {
  push();
  var $$store_subs;
  let pref;
  let newSelect;
  let newTag;
  let selectedItems = /* @__PURE__ */ new Set();
  let selectedTags = /* @__PURE__ */ new Set();
  pref = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  $$payload.out += `<div class="page-columns svelte-x1ifti"><div><div class="block svelte-x1ifti"><div class="header"><h2>Audiosuite Tags</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: "18" });
  $$payload.out += `<!----> Remove</button></div> <div class="bar"><button class="cta-button small">Add</button> <input type="text" id="find-text"${attr("value", newTag)} placeholder="New Tag" class="input-field"></div> <div class="block inner svelte-x1ifti">`;
  VirtualList($$payload, {
    items: Array.from(pref.tags),
    children: invalid_default_snippet,
    $$slots: {
      default: ($$payload2, { item }) => {
        $$payload2.out += `<div${attr("class", to_class("list-item", null, {
          "selected-item": selectedTags.has(item),
          "unselected-item": !selectedTags.has(item)
        }))}>${escape_html(item)}</div>`;
      }
    }
  });
  $$payload.out += `<!----></div></div></div> <div class="arrow-column svelte-x1ifti"><div class="move-button-container svelte-x1ifti"><button class="arrow-button svelte-x1ifti">`;
  ArrowBigRight($$payload, { size: "100" });
  $$payload.out += `<!----></button> <span>`;
  if (selectedItems.size === 0 && selectedTags.size == 0) {
    $$payload.out += "<!--[-->";
    $$payload.out += `<select class="select-field" style="font-size: 10px; width: 100px; margin-left: -1px; text-align: center; text-align-last: center;"><option${attr("value", true)}>Move All</option><option${attr("value", false)}>Copy All</option></select>`;
  } else {
    $$payload.out += "<!--[!-->";
    $$payload.out += `<select class="select-field" style="font-size: 10px; width: 100px; margin-left: -1px; text-align: center; text-align-last: center;"><option${attr("value", true)}>Move Selected</option><option${attr("value", false)}>Copy Selected</option></select>`;
  }
  $$payload.out += `<!--]--></span> <button class="arrow-button svelte-x1ifti">`;
  ArrowBigLeft($$payload, { size: "100" });
  $$payload.out += `<!----></button></div></div> <div><div class="block svelte-x1ifti"><div class="header"><h2>Filename Tags</h2> <button class="cta-button cancel">`;
  OctagonX($$payload, { size: "18" });
  $$payload.out += `<!----> Remove</button></div> <div class="bar"><button class="cta-button small">Add</button> <input type="text" id="find-text"${attr("value", newSelect)} placeholder="Add New String" class="input-field"></div> <div class="block inner svelte-x1ifti">`;
  VirtualList($$payload, {
    items: Array.from(pref.autoselects),
    children: invalid_default_snippet,
    $$slots: {
      default: ($$payload2, { item }) => {
        $$payload2.out += `<div${attr("class", to_class("list-item", null, {
          "selected-item": selectedItems.has(item),
          "unselected-item": !selectedItems.has(item)
        }))}>${escape_html(item)}</div>`;
      }
    }
  });
  $$payload.out += `<!----></div></div></div></div>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function Colors($$payload, $$props) {
  push();
  var $$store_subs;
  let preferences;
  const colorVariables = [
    { key: "primaryBg", label: "Primary Background" },
    {
      key: "secondaryBg",
      label: "Secondary Background"
    },
    { key: "textColor", label: "Text Color" },
    { key: "topbarColor", label: "Topbar Color" },
    { key: "accentColor", label: "Accent Color" },
    { key: "hoverColor", label: "Hover Color" },
    { key: "warningColor", label: "Warning Color" },
    {
      key: "warningHover",
      label: "Warning Hover Color"
    },
    { key: "inactiveColor", label: "Inactive Color" }
  ];
  function getCurrentColor(colorKey) {
    return preferences?.colors[colorKey] || "";
  }
  preferences = store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  const each_array = ensure_array_like(colorVariables);
  $$payload.out += `<div class="block svelte-1r8qzsg"><div class="header"><h2>Colors</h2> <button class="cta-button cancel">Reset Defaults</button></div> <div class="bar"></div> <div class="block inner svelte-1r8qzsg"><div class="color-grid svelte-1r8qzsg"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let { key, label } = each_array[$$index];
    $$payload.out += `<div class="color-item svelte-1r8qzsg"><div class="color-details svelte-1r8qzsg"><span class="color-label svelte-1r8qzsg">${escape_html(label)}</span></div> <div class="color-swatch svelte-1r8qzsg"><div class="swatch-box svelte-1r8qzsg"${attr("style", `background-color: ${stringify(getCurrentColor(key))};`)}><input type="color"${attr("value", getCurrentColor(key))} class="color-input svelte-1r8qzsg"></div> <div class="hex-value svelte-1r8qzsg">${escape_html(getCurrentColor(key))}</div></div></div>`;
  }
  $$payload.out += `<!--]--></div></div></div> z`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  pop();
}
function _page($$payload, $$props) {
  push();
  var $$store_subs;
  let presets;
  let newPreset;
  let activeTab = fallback($$props["activeTab"], "matchCriteria");
  console.log("Active Tab:", activeTab);
  store_get($$store_subs ??= {}, "$preferencesStore", preferencesStore);
  presets = store_get($$store_subs ??= {}, "$PresetsStore", PresetsStore);
  const each_array = ensure_array_like(presets);
  $$payload.out += `<div class="app-container svelte-affvot"><div class="top-bar svelte-affvot"><div class="top-bar-left svelte-affvot"><button${attr("class", `nav-link ${stringify(activeTab === "matchCriteria" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
  ListCheck($$payload, { size: 18 });
  $$payload.out += `<!----> <span>Match Criteria</span></div></button> <button${attr("class", `nav-link ${stringify(activeTab === "preservationOrder" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
  ListOrdered($$payload, { size: 18 });
  $$payload.out += `<!----> <span>Preservation Order</span></div></button> <button${attr("class", `nav-link ${stringify(activeTab === "Tags Editor" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
  Tags$1($$payload, { size: 18 });
  $$payload.out += `<!----> <span>Tags Manager</span></div></button> <button${attr("class", `nav-link ${stringify(activeTab === "colors" ? "active" : "")}`)}><div class="flex items-center gap-2">`;
  Palette($$payload, { size: 18 });
  $$payload.out += `<!----> <span>Colors</span></div></button></div></div> <main class="content svelte-affvot" style="margin-bottom: 0px"><div>`;
  if (activeTab === "mainPref") {
    $$payload.out += "<!--[-->";
    Main($$payload);
  } else {
    $$payload.out += "<!--[!-->";
    if (activeTab === "matchCriteria") {
      $$payload.out += "<!--[-->";
      Match($$payload);
    } else {
      $$payload.out += "<!--[!-->";
      if (activeTab === "preservationOrder") {
        $$payload.out += "<!--[-->";
        Order($$payload);
      } else {
        $$payload.out += "<!--[!-->";
        if (activeTab === "audiosuiteTags") {
          $$payload.out += "<!--[-->";
          Tags($$payload);
        } else {
          $$payload.out += "<!--[!-->";
          if (activeTab === "autoSelect") {
            $$payload.out += "<!--[-->";
            Select($$payload);
          } else {
            $$payload.out += "<!--[!-->";
            if (activeTab === "colors") {
              $$payload.out += "<!--[-->";
              Colors($$payload);
            } else {
              $$payload.out += "<!--[!-->";
            }
            $$payload.out += `<!--]-->`;
          }
          $$payload.out += `<!--]-->`;
        }
        $$payload.out += `<!--]-->`;
      }
      $$payload.out += `<!--]-->`;
    }
    $$payload.out += `<!--]-->`;
  }
  $$payload.out += `<!--]--></div> <div class="bar svelte-affvot" style="width: calc(100% + 40px); margin-top: 16px; margin-left: -20px; margin-right: 20px;"><button class="cta-button small" style="margin-left: 30px">Save:</button> <input type="text" class="input-field" placeholder="Enter New Configuraion Preset Name" style="margin-right: 10px;"${attr("value", newPreset)}> <select class="select-field" style="margin-right: 10px"><!--[-->`;
  for (let $$index = 0, $$length = each_array.length; $$index < $$length; $$index++) {
    let p = each_array[$$index];
    $$payload.out += `<option${attr("value", p.name)}>${escape_html(p.name)}</option>`;
  }
  $$payload.out += `<!--]--></select> <button class="cta-button small cancel" style="margin-right: 25px;">Delete</button></div></main></div> <footer>TESTING</footer>`;
  if ($$store_subs) unsubscribe_stores($$store_subs);
  bind_props($$props, { activeTab });
  pop();
}
export {
  _page as default
};
