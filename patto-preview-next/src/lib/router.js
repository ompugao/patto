'use client';

import { useState, useEffect, useCallback } from 'react';

export function useClientRouter() {
  const [currentNote, setCurrentNote] = useState(null);
  const [anchor, setAnchor] = useState(null);

  // Parse URL parameters
  const parseUrl = useCallback(() => {
    if (typeof window === 'undefined') return;
    
    const urlParams = new URLSearchParams(window.location.search);
    const noteParam = urlParams.get('note');
    const hashAnchor = window.location.hash ? window.location.hash.substring(1) : null;
    
    setCurrentNote(noteParam);
    setAnchor(hashAnchor);
  }, []);

  // Navigate to a note
  const navigate = useCallback((notePath, anchorId = null) => {
    if (typeof window === 'undefined') return;
    
    const url = new URL(window.location);
    
    if (notePath) {
      url.searchParams.set('note', notePath);
    } else {
      url.searchParams.delete('note');
    }
    
    if (anchorId) {
      url.hash = `#${anchorId}`;
    } else {
      url.hash = '';
    }
    
    // Update URL without page reload
    window.history.pushState({ note: notePath, anchor: anchorId }, '', url.toString());
    
    setCurrentNote(notePath);
    setAnchor(anchorId);
  }, []);

  // Navigate to home (clear note)
  const navigateHome = useCallback(() => {
    navigate(null, null);
  }, [navigate]);

  // Handle browser back/forward and initial load
  useEffect(() => {
    if (typeof window === 'undefined') return;
    
    // Parse initial URL
    parseUrl();
    
    // Listen for browser navigation
    const handlePopState = () => {
      parseUrl();
    };
    
    const handleHashChange = () => {
      const hashAnchor = window.location.hash ? window.location.hash.substring(1) : null;
      setAnchor(hashAnchor);
    };
    
    window.addEventListener('popstate', handlePopState);
    window.addEventListener('hashchange', handleHashChange);
    
    return () => {
      window.removeEventListener('popstate', handlePopState);
      window.removeEventListener('hashchange', handleHashChange);
    };
  }, [parseUrl]);

  return {
    currentNote,
    anchor,
    navigate,
    navigateHome
  };
}