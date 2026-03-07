'use client';

import { useEffect, useState, useRef } from 'react';

interface TypewriterProps {
  phrases: string[];
  className?: string;
}

export function Typewriter({ phrases, className = '' }: TypewriterProps) {
  const [displayText, setDisplayText] = useState('');
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [isDeleting, setIsDeleting] = useState(false);
  const timeoutRef = useRef<NodeJS.Timeout | null>(null);

  const TYPING_SPEED = 80;
  const DELETING_SPEED = 40;
  const PAUSE_DURATION = 2000;

  useEffect(() => {
    const currentPhrase = phrases[phraseIndex];

    const handleTyping = () => {
      if (!isDeleting) {
        if (displayText.length < currentPhrase.length) {
          setDisplayText(currentPhrase.slice(0, displayText.length + 1));
          timeoutRef.current = setTimeout(handleTyping, TYPING_SPEED);
        } else {
          timeoutRef.current = setTimeout(() => {
            setIsDeleting(true);
          }, PAUSE_DURATION);
        }
      } else {
        if (displayText.length > 0) {
          setDisplayText(displayText.slice(0, -1));
          timeoutRef.current = setTimeout(handleTyping, DELETING_SPEED);
        } else {
          setIsDeleting(false);
          setPhraseIndex((prev) => (prev + 1) % phrases.length);
        }
      }
    };

    timeoutRef.current = setTimeout(handleTyping, isDeleting ? DELETING_SPEED : TYPING_SPEED);

    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, [displayText, phraseIndex, isDeleting, phrases]);

  return (
    <span className={className}>
      {displayText}
      <span className="typewriter-cursor" />
    </span>
  );
}
