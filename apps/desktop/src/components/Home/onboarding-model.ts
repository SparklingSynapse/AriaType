import type { RecommendedModel } from "@/lib/tauri";

export function resolveOnboardingModelReady({
  selectedModel,
  models,
  progressMap,
  downloadedMap,
}: {
  selectedModel: string | null;
  models: RecommendedModel[];
  progressMap: Record<string, number>;
  downloadedMap: Record<string, boolean>;
}): boolean {
  if (!selectedModel) {
    return false;
  }

  const model = models.find((entry) => entry.model_name === selectedModel);
  const progress = progressMap[selectedModel] ?? 0;

  return downloadedMap[selectedModel] === true || model?.downloaded === true || progress >= 100;
}

export function resolveOnboardingModelProgress({
  selectedModel,
  models,
  progressMap,
  downloadedMap,
}: {
  selectedModel: string | null;
  models: RecommendedModel[];
  progressMap: Record<string, number>;
  downloadedMap: Record<string, boolean>;
}): number {
  if (
    resolveOnboardingModelReady({
      selectedModel,
      models,
      progressMap,
      downloadedMap,
    })
  ) {
    return 100;
  }

  if (!selectedModel) {
    return 0;
  }

  return progressMap[selectedModel] ?? 0;
}
