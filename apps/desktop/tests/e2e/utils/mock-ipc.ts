import { readFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const MOCK_DIR = join(__dirname, '../mocks');

const DEFAULT_MOCK_DATA = {
  settings: {
    shortcut_profiles: {
      dictate: { hotkey: 'Cmd+Slash', trigger_mode: 'hold', action: { Record: { polish_template_id: null } } },
      riff: { hotkey: 'Opt+Slash', trigger_mode: 'toggle', action: { Record: { polish_template_id: 'filler' } } },
      custom: null,
    },
    recording_mode: 'hold',
    stt_engine: 'whisper',
    stt_engine_language: 'auto',
    model: 'whisper-base',
    stt_engine_work_domain: 'general',
    polish_model: '',
    polish_custom_templates: [],
    cloud_stt_enabled: false,
    cloud_polish_enabled: false,
    theme_mode: 'system',
  },
  models: [
    { model_name: 'whisper-base', downloaded: true, size_mb: 74 },
    { model_name: 'whisper-small', downloaded: false, size_mb: 244 },
    { model_name: 'sense-voice-small', downloaded: false, size_mb: 150 },
  ],
  history: [],
  dashboardStats: {
    total_recordings: 27,
    total_duration_seconds: 3600,
    avg_accuracy: 0.89,
    active_days: 27,
  },
  shortcutProfiles: {
    dictate: { hotkey: 'Cmd+Slash', trigger_mode: 'hold', action: { Record: { polish_template_id: null } } },
    riff: { hotkey: 'Opt+Slash', trigger_mode: 'toggle', action: { Record: { polish_template_id: 'filler' } } },
    custom: null,
  },
};

function loadMockData() {
  try {
    if (!existsSync(MOCK_DIR)) {
      console.warn('[Mock IPC] Mock directory not found at', MOCK_DIR, ', using default data');
      return DEFAULT_MOCK_DATA;
    }

    const settings = existsSync(join(MOCK_DIR, 'settings.json'))
      ? JSON.parse(readFileSync(join(MOCK_DIR, 'settings.json'), 'utf-8'))
      : DEFAULT_MOCK_DATA.settings;
    
    const models = existsSync(join(MOCK_DIR, 'models.json'))
      ? JSON.parse(readFileSync(join(MOCK_DIR, 'models.json'), 'utf-8'))
      : DEFAULT_MOCK_DATA.models;
    
    const history = existsSync(join(MOCK_DIR, 'history.json'))
      ? JSON.parse(readFileSync(join(MOCK_DIR, 'history.json'), 'utf-8'))
      : DEFAULT_MOCK_DATA.history;
    
    const dashboardStats = existsSync(join(MOCK_DIR, 'dashboard-stats.json'))
      ? JSON.parse(readFileSync(join(MOCK_DIR, 'dashboard-stats.json'), 'utf-8'))
      : DEFAULT_MOCK_DATA.dashboardStats;
    
    const shortcutProfiles = existsSync(join(MOCK_DIR, 'shortcut-profiles.json'))
      ? JSON.parse(readFileSync(join(MOCK_DIR, 'shortcut-profiles.json'), 'utf-8'))
      : DEFAULT_MOCK_DATA.shortcutProfiles;

    console.log('[Mock IPC] Loaded mock data from', MOCK_DIR);
    return { settings, models, history, dashboardStats, shortcutProfiles };
  } catch (e) {
    console.warn('[Mock IPC] Failed to load mock files, using default data:', e);
    return DEFAULT_MOCK_DATA;
  }
}

export const mockData = loadMockData();

export function generateMockIPCScript(): string {
  const data = mockData;
  
  return `
    (function() {
      if (window.__TAURI_INTERNALS__) return;
      
      window.__TAURI_INTERNALS__ = {
        invoke: async (cmd, args) => {
          console.log('[Mock IPC]', cmd, args);
          
          switch (cmd) {
            case 'get_settings':
              return ${JSON.stringify(data.settings)};
            case 'get_models':
              return ${JSON.stringify(data.models)};
            case 'get_transcription_history':
              return ${JSON.stringify(data.history)};
            case 'get_dashboard_stats':
              return ${JSON.stringify(data.dashboardStats)};
            case 'get_shortcut_profiles':
              return ${JSON.stringify(data.shortcutProfiles)};
            case 'get_daily_usage':
              return [];
            case 'get_engine_usage':
              return [];
            case 'update_settings':
              return null;
            case 'check_permission':
              return 'granted';
            case 'get_platform':
              return 'macos';
            case 'get_audio_devices':
              return ['default'];
            case 'get_log_content':
              return '[INFO] Application started\\n[INFO] Mock log content\\n[DEBUG] Test log entry';
            case 'get_glossary_content':
              return '';
            case 'get_available_subdomains':
              return [];
            case 'get_cloud_provider_schemas':
              return { stt: [], polish: [] };
            case 'get_polish_templates':
              return [
                { id: 'filler', name: 'Remove Fillers', system_prompt: 'Remove filler words.' },
                { id: 'formal', name: 'Make Formal', system_prompt: 'Make text formal.' },
              ];
            case 'get_polish_custom_templates':
              return [];
            case 'update_shortcut_profile':
            case 'create_custom_profile':
            case 'delete_custom_profile':
            case 'start_hotkey_capture':
            case 'stop_hotkey_capture':
            case 'cancel_hotkey_capture':
              return null;
            default:
              console.warn('[Mock IPC] Unknown command:', cmd);
              return null;
          }
        }
      };
      
      console.log('[Mock IPC] Initialized');
    })();
  `;
}
