console.log('Loading module:', 'types.ts');  // Add to each file


// src/stores/types.ts
export type HashMap = Record<string, string>;

export type Metadata = { 
    find: string; 
    replace: string; 
    column: string; 
    case_sensitive: boolean; 
    mark_dirty: boolean };


export type Algorithm = { 
  id: string; 
  name: string; 
  enabled: boolean; 
  min_dur?: number;
  db?: string | null 
};

export type Registration = { 
  name: string; 
  email: string; 
  license: string;
};

export type PreservationLogic = { 
  column: string; 
  operator: string; 
  variable: string 
};

export type Colors = {
  primaryBg: string;
  secondaryBg: string;
  textColor: string;
  topbarColor: string;
  accentColor: string;
  hoverColor: string;
  warningColor: string;
  warningHover: string;
  inactiveColor: string;
};

export type FileRecord = { 
    filename: string; 
    path: string; 
    algorithm: string[]; 
    id: number; 
    duration: string; 
    samplerate: string; 
    bitdepth: string; 
    channels: string; 
    description: string; };


export type Database = {
    path: string;
    name: string | null;
    size: number;
    columns: string[];
    isLoading: boolean;
    error: null | string;
    // records: FileRecord[];
}

export type Preferences = { 
  firstOpen: boolean;
  showToolbars: boolean;
  match_criteria: string[]; 
  ignore_filetype: boolean; 
  autoselects: string[]; 
  tags: string[];
  preservation_order: PreservationLogic[];
  columns: string[];
  display_all_records: boolean;
  safety_db: boolean;
  safety_db_tag: string;
  erase_files: string;
  strip_dual_mono: boolean;
  waveform_search_type: string;
  similarity_threshold: number;
  store_waveforms: boolean;
  fetch_waveforms: boolean;
  colors: Colors;
  algorithms: Algorithm[];
};

export type Preset = { 
  name: string; 
  pref: Preferences 
};

export type SearchProgressState = {
    searchProgress: number;
    searchMessage: string;
    searchStage: string;
    subsearchProgress: number;
    subsearchMessage: string;
    subsearchStage: string;
};