// CSS Assets
import './style.css';

import { diff } from '../pkg';

// ── Utilities ────────────────────────────────────────

const debounce = (fn, delay) => {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), delay);
  };
};

const readFile = (file) => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result);
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file);
  });
};

const parseErrorLine = (message) => {
  const match = message.match(/at line:\s*(\d+)/);
  return match ? parseInt(match[1], 10) : null;
};

// ── Gutter ───────────────────────────────────────────

const updateGutter = (textarea, gutter, errorLine = null) => {
  const lineCount = textarea.value.split('\n').length;
  let html = '';
  for (let i = 1; i <= lineCount; i++) {
    const cls = i === errorLine ? 'gutter-line gutter-line--error' : 'gutter-line';
    html += `<span class="${cls}">${i}</span>`;
  }
  gutter.innerHTML = html;
};

const syncScroll = (textarea, gutter) => {
  gutter.scrollTop = textarea.scrollTop;
};

// ── File Upload ──────────────────────────────────────

const setupFileUpload = (fileInput, textarea, gutter, onContentChange) => {
  fileInput.addEventListener('change', async (event) => {
    const file = event.target.files[0];
    if (!file) return;
    try {
      const text = await readFile(file);
      textarea.value = text;
      try {
        localStorage.setItem(textarea.id, text);
      } catch (storageError) {
        if (storageError.name === 'QuotaExceededError') {
          console.warn(
            'localStorage quota exceeded. Content will not be persisted across sessions.',
          );
        } else {
          console.error('localStorage error:', storageError);
        }
      }
      updateGutter(textarea, gutter);
      onContentChange();
    } catch (e) {
      console.error('File read error:', e);
    }
    fileInput.value = '';
  });
};

// ── Error Display ────────────────────────────────────

const showError = (errorDiv, gutter, textarea, message, label) => {
  const match = message.match(new RegExp(`\\[${label}\\][^\\n]*`));
  if (match) {
    const errorText = match[0].replace(`[${label}]`, '').trim();
    errorDiv.textContent = errorText;
    errorDiv.classList.remove('hidden');
    const errorLine = parseErrorLine(match[0]);
    updateGutter(textarea, gutter, errorLine);
  }
};

const clearError = (errorDiv, gutter, textarea) => {
  errorDiv.classList.add('hidden');
  errorDiv.textContent = '';
  updateGutter(textarea, gutter);
};

// ── Diff Tree Rendering ─────────────────────────────

const DIFF_TYPE = {
  UNCHANGED: 0,
  ADDITIONS: 1,
  DELETIONS: 2,
  MODIFIED: 3,
};

const formatValue = (value) => {
  if (value === null || value === undefined) return 'null';
  if (typeof value === 'string') return `"${value}"`;
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
};

const countDiffs = (diffs) => {
  let additions = 0;
  let deletions = 0;
  let modified = 0;

  const walk = (nodes) => {
    for (const node of nodes) {
      if (node.children.length > 0) {
        walk(node.children);
      } else if (node.has_diff) {
        switch (node.diff_type) {
          case DIFF_TYPE.ADDITIONS:
            additions++;
            break;
          case DIFF_TYPE.DELETIONS:
            deletions++;
            break;
          case DIFF_TYPE.MODIFIED:
            modified++;
            break;
        }
      }
    }
  };

  walk(diffs);
  return { additions, deletions, modified };
};

const diffTypeClass = (diffType) => {
  switch (diffType) {
    case DIFF_TYPE.ADDITIONS:
      return 'diff-row--addition';
    case DIFF_TYPE.DELETIONS:
      return 'diff-row--deletion';
    case DIFF_TYPE.MODIFIED:
      return 'diff-row--modified';
    default:
      return 'diff-row--unchanged';
  }
};

const renderDiffValue = (node) => {
  const container = document.createElement('span');
  container.className = 'diff-value';

  switch (node.diff_type) {
    case DIFF_TYPE.ADDITIONS: {
      container.textContent = formatValue(node.diff.right_value);
      break;
    }
    case DIFF_TYPE.DELETIONS: {
      container.textContent = formatValue(node.diff.left_value);
      break;
    }
    case DIFF_TYPE.MODIFIED: {
      const left = document.createElement('span');
      left.className = 'diff-value--left';
      left.textContent = formatValue(node.diff.left_value);

      const arrow = document.createElement('span');
      arrow.className = 'diff-arrow';
      arrow.textContent = '\u2192';

      const right = document.createElement('span');
      right.className = 'diff-value--right';
      right.textContent = formatValue(node.diff.right_value);

      container.append(left, arrow, right);
      break;
    }
    case DIFF_TYPE.UNCHANGED: {
      container.textContent = formatValue(node.diff.left_value);
      break;
    }
  }

  return container;
};

const isSingleScalar = (node) =>
  node.children.length === 1 &&
  node.children[0].children.length === 0 &&
  (node.children[0].key === undefined || node.children[0].key === null);

const nodeMatchesFilter = (node, filter) => {
  if (filter === null) return true;
  if (isSingleScalar(node)) return node.children[0].diff_type === filter;
  if (node.children.length > 0) {
    return node.children.some((child) => nodeMatchesFilter(child, filter));
  }
  return node.diff_type === filter;
};

const renderDiffNode = (node, isRoot = false, filter = null) => {
  if (!nodeMatchesFilter(node, filter)) return null;

  const hasChildren = node.children.length > 0;
  const hasKey = node.key !== undefined && node.key !== null;

  if (hasChildren) {
    // Scalar value wrapped in a single bare-value child — render inline
    if (isSingleScalar(node)) {
      const child = node.children[0];
      const div = document.createElement('div');
      div.className = `diff-row ${diffTypeClass(child.diff_type)}`;
      if (hasKey) {
        const keySpan = document.createElement('span');
        keySpan.className = 'diff-key';
        keySpan.textContent = node.key + ':';
        div.appendChild(keySpan);
      }
      div.appendChild(renderDiffValue(child));

      if (isRoot) {
        div.classList.add('diff-node-root');
        return div;
      }
      const wrapper = document.createElement('div');
      wrapper.className = 'diff-node';
      wrapper.appendChild(div);
      return wrapper;
    }

    const details = document.createElement('details');
    details.className = `diff-node${isRoot ? ' diff-node-root' : ''}`;

    if (node.has_diff) {
      details.open = true;
    }

    const summary = document.createElement('summary');
    summary.className = diffTypeClass(node.diff_type);
    if (hasKey) {
      const keySpan = document.createElement('span');
      keySpan.className = 'diff-key';
      keySpan.textContent = node.key + ':';
      summary.appendChild(keySpan);
    }
    details.appendChild(summary);

    for (const child of node.children) {
      const childEl = renderDiffNode(child, false, filter);
      if (childEl) details.appendChild(childEl);
    }

    return details;
  }

  const div = document.createElement('div');
  div.className = `diff-row ${diffTypeClass(node.diff_type)}${isRoot ? ' diff-node-root' : ''}`;

  if (hasKey) {
    const keySpan = document.createElement('span');
    keySpan.className = 'diff-key';
    keySpan.textContent = node.key + ':';
    div.appendChild(keySpan);
  }

  div.appendChild(renderDiffValue(node));
  return div;
};

const renderDiffTree = (diffs, container, summaryEls, filter = null) => {
  const counts = countDiffs(diffs);
  summaryEls.additions.textContent = counts.additions;
  summaryEls.deletions.textContent = counts.deletions;
  summaryEls.modified.textContent = counts.modified;

  const treeEl = container.querySelector('#diff-tree');
  treeEl.innerHTML = '';

  for (const node of diffs) {
    const el = renderDiffNode(node, true, filter);
    if (el) treeEl.appendChild(el);
  }

  container.classList.remove('hidden');
};

// ── Initialization ───────────────────────────────────

const ready = (fn) => {
  if (document.readyState !== 'loading') {
    fn();
  } else {
    document.addEventListener('DOMContentLoaded', fn);
  }
};

const supported = () => {
  try {
    if (typeof WebAssembly === 'object' && typeof WebAssembly.instantiate === 'function') {
      const module = new WebAssembly.Module(
        Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00),
      );
      if (module instanceof WebAssembly.Module)
        return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
    }
  } catch (e) {
    console.error(e);
  }
  return false;
};

ready(() => {
  if (!supported()) {
    const appContainer = document.getElementById('app-container');
    const appError = document.getElementById('app-error');
    appContainer.classList.add('hidden');
    appError.classList.remove('hidden');
    return;
  }

  const textAreaOne = document.getElementById('text-area-one');
  const textAreaTwo = document.getElementById('text-area-two');
  const gutterOne = document.getElementById('gutter-one');
  const gutterTwo = document.getElementById('gutter-two');
  const errorOne = document.getElementById('yaml-one-error');
  const errorTwo = document.getElementById('yaml-two-error');
  const fileOne = document.getElementById('file-one');
  const fileTwo = document.getElementById('file-two');
  const diffContainer = document.getElementById('diff-container');
  const summaryEls = {
    additions: document.getElementById('diff-additions'),
    deletions: document.getElementById('diff-deletions'),
    modified: document.getElementById('diff-modified'),
  };
  const filterBtns = {
    additions: document.getElementById('filter-additions'),
    deletions: document.getElementById('filter-deletions'),
    modified: document.getElementById('filter-modified'),
  };

  let lastDiffData = null;
  let activeFilter = null;

  // Restore from localStorage
  textAreaOne.value = localStorage.getItem('text-area-one') || '';
  textAreaTwo.value = localStorage.getItem('text-area-two') || '';

  // Initial gutter render
  updateGutter(textAreaOne, gutterOne);
  updateGutter(textAreaTwo, gutterTwo);

  // Scroll sync
  textAreaOne.addEventListener('scroll', () => syncScroll(textAreaOne, gutterOne));
  textAreaTwo.addEventListener('scroll', () => syncScroll(textAreaTwo, gutterTwo));

  // Filter helpers
  const clearActiveFilterBtn = () => {
    filterBtns.additions.classList.remove('diff-filter--active');
    filterBtns.deletions.classList.remove('diff-filter--active');
    filterBtns.modified.classList.remove('diff-filter--active');
  };

  // Core diff
  const runDiff = () => {
    clearError(errorOne, gutterOne, textAreaOne);
    clearError(errorTwo, gutterTwo, textAreaTwo);

    const v1 = textAreaOne.value;
    const v2 = textAreaTwo.value;

    if (!v1.trim() && !v2.trim()) {
      diffContainer.classList.add('hidden');
      return;
    }

    try {
      const diffData = diff(v1, v2);
      lastDiffData = diffData;
      activeFilter = null;
      clearActiveFilterBtn();
      renderDiffTree(diffData, diffContainer, summaryEls);
    } catch (e) {
      diffContainer.classList.add('hidden');
      if (e.message.includes('[YAML ONE]')) {
        showError(errorOne, gutterOne, textAreaOne, e.message, 'YAML ONE');
      }
      if (e.message.includes('[YAML TWO]')) {
        showError(errorTwo, gutterTwo, textAreaTwo, e.message, 'YAML TWO');
      }
    }
  };

  const debouncedDiff = debounce(runDiff, 400);

  // Filter buttons
  const handleFilterClick = (event) => {
    const btn = event.currentTarget;
    const filterType = parseInt(btn.dataset.filter, 10);

    if (activeFilter === filterType) {
      activeFilter = null;
      btn.classList.remove('diff-filter--active');
    } else {
      activeFilter = filterType;
      clearActiveFilterBtn();
      btn.classList.add('diff-filter--active');
    }

    if (lastDiffData) {
      renderDiffTree(lastDiffData, diffContainer, summaryEls, activeFilter);
    }
  };

  filterBtns.additions.addEventListener('click', handleFilterClick);
  filterBtns.deletions.addEventListener('click', handleFilterClick);
  filterBtns.modified.addEventListener('click', handleFilterClick);

  // Input events
  textAreaOne.addEventListener('input', (event) => {
    try {
      localStorage.setItem('text-area-one', event.target.value);
    } catch (storageError) {
      if (storageError.name === 'QuotaExceededError') {
        console.warn('localStorage quota exceeded. Content will not be persisted across sessions.');
      } else {
        console.error('localStorage error:', storageError);
      }
    }
    updateGutter(textAreaOne, gutterOne);
    debouncedDiff();
  });

  textAreaTwo.addEventListener('input', (event) => {
    try {
      localStorage.setItem('text-area-two', event.target.value);
    } catch (storageError) {
      if (storageError.name === 'QuotaExceededError') {
        console.warn('localStorage quota exceeded. Content will not be persisted across sessions.');
      } else {
        console.error('localStorage error:', storageError);
      }
    }
    updateGutter(textAreaTwo, gutterTwo);
    debouncedDiff();
  });

  // File uploads (immediate diff)
  setupFileUpload(fileOne, textAreaOne, gutterOne, runDiff);
  setupFileUpload(fileTwo, textAreaTwo, gutterTwo, runDiff);

  // Tab key → insert 2 spaces
  const handleTab = (event) => {
    if (event.key === 'Tab') {
      event.preventDefault();
      const ta = event.target;
      const start = ta.selectionStart;
      const end = ta.selectionEnd;
      ta.value = ta.value.substring(0, start) + '  ' + ta.value.substring(end);
      ta.selectionStart = ta.selectionEnd = start + 2;
      ta.dispatchEvent(new Event('input'));
    }
  };

  textAreaOne.addEventListener('keydown', handleTab);
  textAreaTwo.addEventListener('keydown', handleTab);

  // Initial diff if content exists
  if (textAreaOne.value.trim() || textAreaTwo.value.trim()) {
    runDiff();
  }
});
