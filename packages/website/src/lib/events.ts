export const AnalyticsEvents = {
  PAGE_VIEW: 'website_page_view',
  CTA_CLICK: 'website_cta_click',
  NAV_CLICK: 'website_nav_click',
  LANGUAGE_SWITCH: 'website_language_switch',
  FOOTER_LINK_CLICK: 'website_footer_link_click',
  DOWNLOAD_CLICK: 'website_download_click',
} as const;

export type AnalyticsEvent = typeof AnalyticsEvents[keyof typeof AnalyticsEvents];
