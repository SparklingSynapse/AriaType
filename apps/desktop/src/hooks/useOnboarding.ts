import { useState, useEffect, useCallback } from "react";

const ONBOARDING_KEY = "onboarding_completed";

export function useOnboarding() {
  const [isFirstVisit, setIsFirstVisit] = useState<boolean | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    const completed = localStorage.getItem(ONBOARDING_KEY);
    const isFirst = completed !== "true";
    setIsFirstVisit(isFirst);
    if (isFirst) {
      setIsOpen(true);
    }
  }, []);

  const closeOnboarding = useCallback(() => {
    localStorage.setItem(ONBOARDING_KEY, "true");
    setIsOpen(false);
  }, []);

  const resetOnboarding = useCallback(() => {
    localStorage.removeItem(ONBOARDING_KEY);
    setIsFirstVisit(true);
    setIsOpen(true);
  }, []);

  return {
    isFirstVisit,
    isOpen,
    closeOnboarding,
    resetOnboarding,
  };
}
