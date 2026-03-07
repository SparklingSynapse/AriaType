export interface Settings {
    autoStart: boolean;
    recordingSound: boolean;
    pillPosition: PillPosition;
    indicatorMode: IndicatorMode;
    selectedModel: string;
    language: string;
}
export type PillPosition = 'top-left' | 'top-center' | 'top-right' | 'bottom-left' | 'bottom-center' | 'bottom-right';
export type IndicatorMode = 'always-show' | 'show-when-recording' | 'never-show';
export interface Model {
    id: string;
    name: string;
    size: string;
    downloaded: boolean;
    downloading: boolean;
    progress?: number;
}
export interface UpdateInfo {
    version: string;
    date?: string;
    notes?: string;
    url?: string;
}
//# sourceMappingURL=types.d.ts.map