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
  if (!response || !response.body) return;
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });

      const lines = buffer.split('\n');
      buffer = lines.pop();

      let streamFinished = false;
      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        if (trimmed === 'data: [DONE]' || trimmed === 'data:[DONE]') {
          streamFinished = true;
          break;
        }
        if (trimmed.startsWith('data: ')) {
          const dataStr = trimmed.slice(6).trim();
          if (dataStr === '[DONE]' || dataStr === '[done]') {
            streamFinished = true;
            break;
          }
          try {
            const payload = JSON.parse(dataStr);
            if (payload.type === 'token' && typeof payload.text === 'string') onToken(payload.text);
            if (onPayload) await onPayload(payload);
          } catch (e) {
            console.warn('Error parsing SSE payload JSON:', dataStr, e);
          }
        }
      }
      if (streamFinished) {
        break;
      }
    }

    if (buffer && buffer.trim()) {
      const trimmed = buffer.trim();
      if (trimmed.startsWith('data: ')) {
        const dataStr = trimmed.slice(6).trim();
        if (dataStr && dataStr !== '[DONE]' && dataStr !== '[done]') {
          try {
            const payload = JSON.parse(dataStr);
            if (payload.type === 'token' && typeof payload.text === 'string') onToken(payload.text);
            if (onPayload) await onPayload(payload);
          } catch (_) {}
        }
      }
    }
  } finally {
    try {
      await reader.cancel();
    } catch (_) {}
  }
}

// Slugify a string for use in filenames (e.g. "Hello World!" → "hello-world")
export function slugify(str) {
  return (str || '').trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '') || 'untitled';
}

// Extract a human-readable title from a file path or URL
export function titleFromPath(path) {
  if (!path) return '';
  try {
    if (path.startsWith('http://') || path.startsWith('https://')) {
      const parsedUrl = new URL(path);
      const parts = parsedUrl.pathname.split('/').filter(p => p.length > 0);
      if (parts.length > 0) {
        const last = parts[parts.length - 1].replace(/\.[^/.]+$/, '').replace(/[-_]+/g, ' ');
        if (last.length > 2) return last.charAt(0).toUpperCase() + last.slice(1);
      }
      if (parsedUrl.hostname) return parsedUrl.hostname;
      return '';
    }
  } catch (_) {}
  const parts = path.split(/[\/\\]/);
  const last = parts[parts.length - 1].replace(/\.[^/.]+$/, '').replace(/[-_]+/g, ' ');
  return last.length > 0 ? last.charAt(0).toUpperCase() + last.slice(1) : '';
}

// Check if a string is an external URL (http, https, mailto, ftp)
export function isExternalUrlStr(str) {
  return /^(https?:\/\/|mailto:|ftp:\/\/)/i.test((str || '').trim());
}

// Compute an .adoc filename from a search query string
export function computeFilenameFromQuery(query) {
  const q = (query || '').trim();
  if (!q) return '';
  if (isExternalUrlStr(q)) return q;
  if (q.toLowerCase().endsWith('.adoc')) return q;
  return slugify(q) + '.adoc';
}

// Build a formatted link preview string (xref: or URL)
export function buildLinkPreview({ filename, title, displayText, query, isExternal, customFilename }) {
  if (filename) {
    const text = (displayText || '').trim() || title || filename;
    return `xref:${filename}[${text}]`;
  }
  const q = (query || '').trim();
  if (q) {
    const text = (displayText || '').trim() || q;
    if (isExternal) return `${q}[${text}]`;
    const fn = customFilename || computeFilenameFromQuery(q);
    return `xref:${fn}[${text}]`;
  }
  return '';
}

// Split AsciiDoc text into blocks respecting delimiters (====, ----, ****, etc.)
export function parseBlocksFromText(text) {
  if (!text || !text.trim()) return [''];
  const rawBlocks = text.split(/\n\s*\n/);
  const blocks = [];
  let currentAcc = '';
  for (const raw of rawBlocks) {
    const b = raw.trim();
    if (!b) continue;
    if (currentAcc) {
      currentAcc += '\n\n' + raw;
      const dm = currentAcc.match(/^(=|--|-|\*|\.|_){4,}|^\|===/gm);
      if (dm && dm.length % 2 === 0) { blocks.push(currentAcc); currentAcc = ''; }
    } else {
      const dm = b.match(/^(=|--|-|\*|\.|_){4,}|^\|===/gm);
      if (dm && dm.length % 2 === 1) currentAcc = raw;
      else blocks.push(raw);
    }
  }
  if (currentAcc) blocks.push(currentAcc);
  return blocks.length > 0 ? blocks : [text];
}

// Format session remaining time as compact string (e.g. "1h 1m")
export function formatSessionRemaining(seconds) {
  if (seconds <= 0) return 'Expired';
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  if (days >= 1) return `${days}d ${hours}h`;
  if (hours >= 1) return `${hours}h ${minutes}m`;
  if (minutes >= 1) return `${minutes}m ${secs}s`;
  return `${secs}s`;
}

// Format session remaining time as verbose string (e.g. "1 hr 2 min")
export function formatSessionRemainingFull(seconds) {
  if (seconds <= 0) return 'Expired';
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);
  const parts = [];
  if (days > 0) parts.push(`${days} day${days > 1 ? 's' : ''}`);
  if (hours > 0) parts.push(`${hours} hr${hours > 1 ? 's' : ''}`);
  if (minutes > 0) parts.push(`${minutes} min`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs} sec`);
  return parts.join(' ');
}
