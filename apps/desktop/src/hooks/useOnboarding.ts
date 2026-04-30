import { useState, useEffect, useCallback } from "react";

const ONBOARDING_KEY = "onboarding_completed";
const ONBOARDING_RESET_EVENT = "ariatype:onboarding-reset";
const ONBOARDING_COMPLETE_EVENT = "ariatype:onboarding-complete";

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

  useEffect(() => {
    const handleReset = () => {
      localStorage.removeItem(ONBOARDING_KEY);
      setIsFirstVisit(true);
      setIsOpen(true);
    };

    const handleComplete = () => {
      localStorage.setItem(ONBOARDING_KEY, "true");
      setIsFirstVisit(false);
      setIsOpen(false);
    };

    window.addEventListener(ONBOARDING_RESET_EVENT, handleReset);
    window.addEventListener(ONBOARDING_COMPLETE_EVENT, handleComplete);

    return () => {
      window.removeEventListener(ONBOARDING_RESET_EVENT, handleReset);
      window.removeEventListener(ONBOARDING_COMPLETE_EVENT, handleComplete);
    };
  }, []);

  const closeOnboarding = useCallback(() => {
    localStorage.setItem(ONBOARDING_KEY, "true");
    setIsFirstVisit(false);
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
