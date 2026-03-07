export const AnalyticsEvents = {
  APP_STARTED: 'desktop_app_started',
  APP_EXITED: 'desktop_app_exited',
  SCREEN_VIEW: 'desktop_screen_view',
  SETTING_CHANGED: 'desktop_setting_changed',
  ONBOARDING_STARTED: 'desktop_onboarding_started',
  ONBOARDING_COMPLETED: 'desktop_onboarding_completed',
  ONBOARDING_SKIPPED: 'desktop_onboarding_skipped',
  ONBOARDING_STEP_VIEWED: 'desktop_onboarding_step_viewed',
  UPDATE_CHECK_STARTED: 'desktop_update_check_started',
  UPDATE_CHECK_COMPLETED: 'desktop_update_check_completed',
  LOG_FOLDER_OPENED: 'desktop_log_folder_opened',
  LOGS_REFRESHED: 'desktop_logs_refreshed',
  PERMISSION_GRANT_REQUESTED: 'desktop_permission_grant_requested',
  RECORDING_STARTED: 'desktop_recording_started',
  RECORDING_STOPPED: 'desktop_recording_stopped',
  RECORDING_STATE_CHANGED: 'desktop_recording_state_changed',
  RECORDING_ERROR: 'desktop_recording_error',
} as const;

export type AnalyticsEvent = typeof AnalyticsEvents[keyof typeof AnalyticsEvents];
