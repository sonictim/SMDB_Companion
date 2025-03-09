import { e as escape_html } from "./escaping.js";
import { clsx as clsx$1 } from "clsx";
import { w as writable } from "./index2.js";
const replacements = {
  translate: /* @__PURE__ */ new Map([
    [true, "yes"],
    [false, "no"]
  ])
};
function attr(name, value, is_boolean = false) {
  if (value == null || !value && is_boolean || value === "" && name === "class") return "";
  const normalized = name in replacements && replacements[name].get(value) || value;
  const assignment = is_boolean ? "" : `="${escape_html(normalized, true)}"`;
  return ` ${name}${assignment}`;
}
function clsx(value) {
  if (typeof value === "object") {
    return clsx$1(value);
  } else {
    return value ?? "";
  }
}
const whitespace = [..." 	\n\r\f \v\uFEFF"];
function to_class(value, hash, directives) {
  var classname = value == null ? "" : "" + value;
  if (hash) {
    classname = classname ? classname + " " + hash : hash;
  }
  if (directives) {
    for (var key in directives) {
      if (directives[key]) {
        classname = classname ? classname + " " + key : key;
      } else if (classname.length) {
        var len = key.length;
        var a = 0;
        while ((a = classname.indexOf(key, a)) >= 0) {
          var b = a + len;
          if ((a === 0 || whitespace.includes(classname[a - 1])) && (b === classname.length || whitespace.includes(classname[b]))) {
            classname = (a === 0 ? "" : classname.substring(0, a)) + classname.substring(b + 1);
          } else {
            a = b;
          }
        }
      }
    }
  }
  return classname === "" ? null : classname;
}
const defaultColors = {
  primaryBg: "#2e3a47",
  // Default value for primary background
  secondaryBg: "#1f2731",
  // Default value for secondary background
  textColor: "#ffffff",
  // Default text color
  topbarColor: "#FFB81C",
  // Default topbar color
  accentColor: "#f0a500",
  // Default accent color
  hoverColor: "#ffcc00",
  // Default hover color
  warningColor: "#b91c1c",
  // Default warning color
  warningHover: "#dc2626",
  // Default warning hover color
  inactiveColor: "#888888"
  // Default inactive color
};
const lightModeColors = {
  primaryBg: "#ffffff",
  // Light gray-white for main background
  secondaryBg: "#ebebeb",
  // Pure white for content areas
  textColor: "#2c3e50",
  // Dark slate for text - good readability
  topbarColor: "#4a90e2",
  // Pleasant blue for top bar
  accentColor: "#3498db",
  // Slightly darker blue for interactive elements
  hoverColor: "#2980b9",
  // Deeper blue for hover states
  warningColor: "#e74c3c",
  // Soft red for warnings
  warningHover: "#c0392b",
  // Deeper red for warning hovers
  inactiveColor: "#bdc3c7"
  // Neutral gray for inactive elements
};
const terminalColors = {
  primaryBg: "#000000",
  // Default value for primary background
  secondaryBg: "#232323",
  // Default value for secondary background
  textColor: "#00f900",
  // Default text color
  topbarColor: "#7a7a7a",
  // Default topbar color
  accentColor: "#00f900",
  // Default accent color
  hoverColor: "#7aff7a",
  // Default hover color
  warningColor: "#000000",
  // Default warning color
  warningHover: "#f90000",
  // Default warning hover color
  inactiveColor: "#7a7a7a"
  // Default inactive color
};
const twilightColors = {
  primaryBg: "#2B3A67",
  // Deep blue-purple, easy on eyes
  secondaryBg: "#3F4B83",
  // Lighter blue-purple for contrast
  textColor: "#E5E9FF",
  // Soft white with slight blue tint
  topbarColor: "#ff8c82",
  // Gold for distinctive top bar
  accentColor: "#FFC145",
  // Warm gold for interactive elements
  hoverColor: "#FFB347",
  // Slightly darker gold for hover states
  warningColor: "#FF6B6B",
  // Soft coral red for warnings
  warningHover: "#FF4949",
  // Brighter coral for warning hovers
  inactiveColor: "#8E9AAF"
  // Muted blue-gray for inactive elements
};
const draculaColors = {
  primaryBg: "#282a36",
  // Dracula background
  secondaryBg: "#44475a",
  // Dracula current line/selection
  textColor: "#f8f8f2",
  // Dracula foreground
  topbarColor: "#ff79c6",
  // Dracula pink
  accentColor: "#bd93f9",
  // Dracula purple
  hoverColor: "#8be9fd",
  // Dracula cyan
  warningColor: "#ff5555",
  // Dracula red
  warningHover: "#ff3333",
  // Brighter red for hover
  inactiveColor: "#6272a4"
  // Dracula comment
};
const nordColors = {
  primaryBg: "#2e3440",
  // Nord Polar Night darkest
  secondaryBg: "#3b4252",
  // Nord Polar Night lighter
  textColor: "#eceff4",
  // Nord Snow Storm lightest
  topbarColor: "#88c0d0",
  // Nord Frost blue
  accentColor: "#81a1c1",
  // Nord Frost darker blue
  hoverColor: "#5e81ac",
  // Nord Frost darkest blue
  warningColor: "#bf616a",
  // Nord Aurora red
  warningHover: "#d08770",
  // Nord Aurora orange-red
  inactiveColor: "#4c566a"
  // Nord Polar Night lightest
};
const tokyoNightColors = {
  primaryBg: "#1a1b26",
  // Tokyo Night background
  secondaryBg: "#24283b",
  // Tokyo Night darker background
  textColor: "#a9b1d6",
  // Tokyo Night foreground
  topbarColor: "#bb9af7",
  // Tokyo Night purple
  accentColor: "#7aa2f7",
  // Tokyo Night blue
  hoverColor: "#2ac3de",
  // Tokyo Night cyan
  warningColor: "#f7768e",
  // Tokyo Night red
  warningHover: "#db4b4b",
  // Tokyo Night dark red
  inactiveColor: "#565f89"
  // Tokyo Night gray
};
const monokaiProColors = {
  primaryBg: "#2d2a2e",
  // Monokai Pro background
  secondaryBg: "#363537",
  // Monokai Pro lighter bg
  textColor: "#fcfcfa",
  // Monokai Pro foreground
  topbarColor: "#ff6188",
  // Monokai Pro pink
  accentColor: "#78dce8",
  // Monokai Pro cyan
  hoverColor: "#a9dc76",
  // Monokai Pro green
  warningColor: "#fc9867",
  // Monokai Pro orange
  warningHover: "#ff6188",
  // Monokai Pro red
  inactiveColor: "#727072"
  // Monokai Pro gray
};
const gruvboxColors = {
  primaryBg: "#282828",
  // Gruvbox dark background
  secondaryBg: "#3c3836",
  // Gruvbox dark1
  textColor: "#ebdbb2",
  // Gruvbox light0
  topbarColor: "#98971a",
  // Gruvbox green (changed from yellow)
  accentColor: "#458588",
  // Gruvbox blue
  hoverColor: "#d79921",
  // Gruvbox yellow
  warningColor: "#cc241d",
  // Gruvbox red
  warningHover: "#fb4934",
  // Gruvbox bright red
  inactiveColor: "#928374"
  // Gruvbox gray
};
const defaultPreferences = {
  display_all_records: true,
  match_criteria: ["Filename", "Channels", "Duration"],
  ignore_filetype: false,
  safety_db: true,
  safety_db_tag: "thinned",
  erase_files: "Keep",
  autoselects: [],
  preservation_order: [
    {
      column: "Description",
      operator: "IsNotEmpty",
      variable: ""
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "Audio Files"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARIES"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARY"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "/LIBRARY"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARY/"
    },
    {
      column: "Duration",
      operator: "Largest",
      variable: ""
    },
    {
      column: "Channels",
      operator: "Largest",
      variable: ""
    },
    {
      column: "SampleRate",
      operator: "Largest",
      variable: ""
    },
    {
      column: "BitDepth",
      operator: "Largest",
      variable: ""
    },
    {
      column: "BWDate",
      operator: "Smallest",
      variable: ""
    },
    {
      column: "ScannedDate",
      operator: "Smallest",
      variable: ""
    }
  ],
  tags: [
    "-1eqa_",
    "-6030_",
    "-7eqa_",
    "-A2sA_",
    "-A44m_",
    "-A44s_",
    "-Alt7S_",
    "-ASMA_",
    "-AVrP_",
    "-AVrT_",
    "-AVSt_",
    "-DUPL_",
    "-DVerb_",
    "-GAIN_",
    "-M2DN_",
    "-NORM_",
    "-NYCT_",
    "-PiSh_",
    "-PnT2_",
    "-PnTPro_",
    "-ProQ2_",
    "-PSh_",
    "-RVRS_",
    "-RX7Cnct_",
    "-spce_",
    "-TCEX_",
    "-TiSh_",
    "-TmShft_",
    "-VariFi_",
    "-VlhllVV_",
    "-VSPD_",
    "-VitmnMn_",
    "-VtmnStr_",
    "-X2mA_",
    "-X2sA_",
    "-XForm_",
    "-Z2N5_",
    "-Z2S5_",
    "-Z4n2_",
    "-ZXN5_"
  ],
  columns: ["AudioFileType", "AuditionLevel", "BWDate", "BWDescription", "BWOriginator", "BWOriginatorRef", "BWTime", "BWTimeStamp", "BitDepth", "Brightness", "CatID", "Category", "CategoryFull", "Category_en", "ChannelLayout", "Channels", "CreationDate", "Description", "Description_en", "Designer", "DesignerInitials", "Duration", "EntryDate", "Era", "FXName", "FilePath", "Filename", "GPSAlt", "GPSLat", "GPSLon", "Index", "Keywords", "LibrarianOnly", "Library", "Location", "LongID", "Manufacturer", "MicPerspective", "Microphone", "ModificationDate", "Notes", "OpenTier", "Pathname", "Popularity", "Rating", "RecMedium", "RecType", "ReleaseDate", "SampleRate", "ScannedDate", "Scene", "ShortID", "Show", "ShowCategory", "ShowDescription", "ShowFXName", "ShowFilename", "ShowLongID", "ShowNotes", "ShowSubCategory", "SoundDesignerOnly", "Source", "SubCategory", "SubCategory_en", "Take", "Tape", "TotalFrames", "TouchedDate", "Track", "TrackTitle", "URL", "Ultrasonics", "UserCategory", "UserComments", "VendorCategory", "Volume", "ixmlCurrentSpeed", "ixmlFileUID", "ixmlMasterSpeed", "ixmlNote", "ixmlOriginalFilename", "ixmlParentFilename", "ixmlParentUID", "ixmlProject", "ixmlSpeedNote", "ixmlTimeCodeFlag", "ixmlTimeCodeRate", "ixmlTrackLayout", "recid"],
  colors: defaultColors
};
const TJFPreferences = {
  ...defaultPreferences,
  colors: terminalColors,
  match_criteria: ["Filename"],
  tags: [...defaultPreferences.tags, "-Reverse_", "-RING_", ".new.", ".wav.", ".mp3.", ".aif."],
  preservation_order: [
    {
      column: "Pathname",
      operator: "Contains",
      variable: "TJF RECORDINGS"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARIES"
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "SHOWS/Tim Farrell"
    },
    {
      column: "Description",
      operator: "IsNotEmpty",
      variable: ""
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "Audio Files"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "RECORD"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "CREATED SFX"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "CREATED FX"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARY"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "/LIBRARY"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "LIBRARY/"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "SIGNATURE"
    },
    {
      column: "Pathname",
      operator: "Contains",
      variable: "PULLS"
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "EDIT"
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "MIX"
    },
    {
      column: "Pathname",
      operator: "DoesNotContain",
      variable: "SESSION"
    },
    {
      column: "Duration",
      operator: "Largest",
      variable: ""
    },
    {
      column: "Channels",
      operator: "Largest",
      variable: ""
    },
    {
      column: "SampleRate",
      operator: "Largest",
      variable: ""
    },
    {
      column: "BitDepth",
      operator: "Largest",
      variable: ""
    },
    {
      column: "BWDate",
      operator: "Smallest",
      variable: ""
    },
    {
      column: "ScannedDate",
      operator: "Smallest",
      variable: ""
    }
  ]
};
const storedPreferences = localStorage.getItem("preferencesInfo");
let initialPreferences;
try {
  const parsedPreferences = storedPreferences ? JSON.parse(storedPreferences) : null;
  initialPreferences = parsedPreferences && Object.keys(parsedPreferences).length > 0 ? parsedPreferences : defaultPreferences;
} catch (e) {
  console.error("Error loading preferences:", e);
  initialPreferences = defaultPreferences;
}
const preferencesStore = writable(initialPreferences);
preferencesStore.subscribe((value) => {
  if (value && Object.keys(value).length > 0) {
    localStorage.setItem("preferencesInfo", JSON.stringify(value));
  } else {
    preferencesStore.set(defaultPreferences);
  }
});
const resultsStore = writable(
  JSON.parse(sessionStorage.getItem("results") || "[]")
);
resultsStore.subscribe((value) => {
  sessionStorage.setItem("results", JSON.stringify(value));
});
const storedPresets = localStorage.getItem("presets");
const storedAlgorithms = localStorage.getItem("selectedAlgorithms");
const defaultAlgorithms = [
  { id: "basic", name: "Duplicate Search", enabled: true },
  { id: "invalidpath", name: "Invalid Files", enabled: false },
  { id: "filename", name: "Similar Filename", enabled: false },
  { id: "duration", name: "Minimum Duration:", enabled: false, min_dur: 0.5 },
  { id: "audiosuite", name: "Audiosuite Tags", enabled: false },
  { id: "filetags", name: "Filename Contains Tag", enabled: false },
  { id: "waveform", name: "Waveform Comparison", enabled: false, db: null },
  { id: "dbcompare", name: "Database Compare:", enabled: false }
];
const algorithmsStore = writable(storedAlgorithms ? JSON.parse(storedAlgorithms) : defaultAlgorithms);
algorithmsStore.subscribe((value) => {
  localStorage.setItem("selectedAlgorithms", JSON.stringify(value));
});
const storedRegistration = localStorage.getItem("registrationInfo");
const defaultReg = { name: "", email: "", license: "" };
const registrationStore = writable(storedRegistration ? JSON.parse(storedRegistration) : defaultReg);
registrationStore.subscribe((value) => {
  localStorage.setItem("registrationInfo", JSON.stringify(value));
});
if (typeof window !== "undefined") {
  window.addEventListener("storage", (event) => {
    if (event.key === "preferencesInfo") {
      const newValue = JSON.parse(event.newValue || "{}");
      preferencesStore.set(newValue);
    }
  });
}
preferencesStore.subscribe((value) => {
  localStorage.setItem("preferencesInfo", JSON.stringify(value));
});
const defaultPresets = [
  { name: "Default", pref: defaultPreferences },
  { name: "TJF", pref: TJFPreferences },
  { name: "Light Mode", pref: { ...defaultPreferences, colors: lightModeColors } },
  { name: "Twilight", pref: { ...defaultPreferences, colors: twilightColors } },
  { name: "Dracula", pref: { ...defaultPreferences, colors: draculaColors } },
  { name: "Nord", pref: { ...defaultPreferences, colors: nordColors } },
  // { name: "One Dark", pref: { ...defaultPreferences, colors: oneDarkColors } },
  { name: "Tokyo Night", pref: { ...defaultPreferences, colors: tokyoNightColors } },
  { name: "Monokai Pro", pref: { ...defaultPreferences, colors: monokaiProColors } },
  { name: "Gruvbox", pref: { ...defaultPreferences, colors: gruvboxColors } }
];
const PresetsStore = writable(storedPresets ? JSON.parse(storedPresets) : defaultPresets);
PresetsStore.subscribe((value) => {
  localStorage.setItem("presets", JSON.stringify(value));
});
export {
  PresetsStore as P,
  attr as a,
  algorithmsStore as b,
  clsx as c,
  preferencesStore as p,
  registrationStore as r,
  to_class as t
};
