// Pure utility functions extracted from app.js
// These have no Vue dependencies and can be used anywhere.

// Markdown-like mini parser for AI chat responses
export function formatMarkdown(text) {
  if (!text || typeof text !== 'string') return '';
  let escaped = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
  escaped = escaped.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, '<pre><code class="lang-$1">$2</code></pre>');
  escaped = escaped.replace(/`([^`]+)`/g, '<code>$1</code>');
  escaped = escaped.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  escaped = escaped.replace(/\*([^*]+)\*/g, '<em>$1</em>');
  escaped = escaped.replace(/\n/g, '<br>');
  return escaped;
}

// Extract slides from AsciiDoc text based on headings (=, ==, ===) and page breaks (<<<)
export function extractSlides(text) {
  if (!text || !text.trim()) {
    return [{ index: 0, title: 'Slide 1', raw: '= Untitled', html: '' }];
  }

  const lines = text.split('\n');
  const slides = [];
  let currentLines = [];

  function flushSlide() {
    if (currentLines.length === 0 && slides.length > 0) return;
    const raw = currentLines.join('\n').trim();
    if (!raw && slides.length > 0) return;

    let title = '';
    for (const l of currentLines) {
      const trimmed = l.trim();
      if (trimmed.startsWith('= ') || trimmed.startsWith('== ') || trimmed.startsWith('=== ') || trimmed.startsWith('==== ')) {
        title = trimmed.replace(/^=+\s+/, '').trim();
        break;
      } else if (!title && trimmed && !trimmed.startsWith('//') && !trimmed.startsWith(':') && !trimmed.startsWith('[')) {
        title = trimmed;
      }
    }
    if (!title) title = `Slide ${slides.length + 1}`;

    slides.push({
      index: slides.length,
      title: title.length > 50 ? title.slice(0, 47) + '...' : title,
      raw: raw || ' ',
      html: ''
    });
    currentLines = [];
  }

  const hasLevel1Or2 = lines.some(l => /^={1,2}\s+\S+/.test(l.trim()));

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === '<<<') { flushSlide(); continue; }

    const isSlideHeading = hasLevel1Or2
      ? (trimmed.startsWith('= ') || trimmed.startsWith('== '))
      : (trimmed.startsWith('= ') || trimmed.startsWith('== ') || trimmed.startsWith('=== '));

    if (isSlideHeading && currentLines.some(l => l.trim().length > 0)) flushSlide();
    currentLines.push(line);
  }

  flushSlide();

  if (slides.length === 0) {
    slides.push({ index: 0, title: 'Slide 1', raw: text, html: '' });
  }

  return slides;
}

// Resolve the initial note filename from URL hash, query, path, or localStorage
export function getRequestedNote() {
  if (typeof window !== 'undefined' && window.location && window.location.hash) {
    let hash = window.location.hash.slice(1).trim();
    if (hash.startsWith('/page/')) hash = hash.slice(6);
    else if (hash.startsWith('page/')) hash = hash.slice(5);
    else if (hash.startsWith('note=')) hash = hash.slice(5);
    try { hash = decodeURIComponent(hash); } catch (_) {}
    if (hash) return hash.endsWith('.adoc') ? hash : hash + '.adoc';
  }

  if (typeof window !== 'undefined' && window.location && window.location.search) {
    try {
      const params = new URLSearchParams(window.location.search);
      const qNote = params.get('note') || params.get('page');
      if (qNote) {
        const decoded = decodeURIComponent(qNote.trim());
        if (decoded) return decoded.endsWith('.adoc') ? decoded : decoded + '.adoc';
      }
    } catch (_) {}
  }

  if (typeof window !== 'undefined' && window.location && window.location.pathname) {
    const pathParts = window.location.pathname.split('/');
    if (pathParts[1] === 'page' || pathParts[1] === 'notes' || pathParts[1] === 'edit') {
      const target = pathParts[2];
      if (target) {
        try {
          const decoded = decodeURIComponent(target.trim());
          return decoded.endsWith('.adoc') ? decoded : decoded + '.adoc';
        } catch (_) {
          return target.endsWith('.adoc') ? target : target + '.adoc';
        }
      }
    }
  }

  try {
    const saved = localStorage.getItem('notesplus_last_note');
    if (saved && saved.trim()) {
      const trimmed = saved.trim();
      return trimmed.endsWith('.adoc') ? trimmed : trimmed + '.adoc';
    }
  } catch (_) {}

  return null;
}

// SSE stream consumer for AI chat responses
export async function consumeSseStream(response, onToken, onPayload) {
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    const lines = buffer.split('\n');
    buffer = lines.pop();

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || trimmed === 'data: [DONE]') continue;
      if (trimmed.startsWith('data: ')) {
        const dataStr = trimmed.slice(6).trim();
        if (dataStr === '[DONE]') continue;
        try {
          const payload = JSON.parse(dataStr);
          if (payload.type === 'token' && typeof payload.text === 'string') onToken(payload.text);
          if (onPayload) await onPayload(payload);
        } catch (e) {
          console.warn('Error parsing SSE payload JSON:', dataStr, e);
        }
      }
    }
  }

  if (buffer && buffer.trim()) {
    const trimmed = buffer.trim();
    if (trimmed.startsWith('data: ')) {
      const dataStr = trimmed.slice(6).trim();
      if (dataStr && dataStr !== '[DONE]') {
        try {
          const payload = JSON.parse(dataStr);
          if (payload.type === 'token' && typeof payload.text === 'string') onToken(payload.text);
          if (onPayload) await onPayload(payload);
        } catch (_) {}
      }
    }
  }
}
