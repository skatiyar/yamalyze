// CSS Assets
import './style.css';

import { diff_init, diff_key, diff_stored, diff_cleanup } from '../pkg';

// ── Utilities ────────────────────────────────────────

const debounce = (fn, delay) => {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), delay);
  };
};

const yieldToUI = () => new Promise((resolve) => setTimeout(resolve, 0));

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

// ── File Size Tiers ─────────────────────────────────

const SIZE_TIERS = {
  SMALL: 'small', // < 1MB — full auto behavior
  MEDIUM: 'medium', // 1-10MB — auto-diff, skip localStorage
  LARGE: 'large', // 10-50MB — explicit diff only
  HUGE: 'huge', // 50-100MB — explicit diff, read-only
  REJECTED: 'rejected', // > 100MB — refuse
};

const getFileTier = (byteSize) => {
  const MB = 1024 * 1024;
  if (byteSize > 100 * MB) return SIZE_TIERS.REJECTED;
  if (byteSize > 50 * MB) return SIZE_TIERS.HUGE;
  if (byteSize > 10 * MB) return SIZE_TIERS.LARGE;
  if (byteSize > 1 * MB) return SIZE_TIERS.MEDIUM;
  return SIZE_TIERS.SMALL;
};

const getContentByteSize = (str) => new TextEncoder().encode(str).length;

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

const setupFileUpload = (fileInput, textarea, gutter, warningDiv, onContentChange) => {
  fileInput.addEventListener('change', async (event) => {
    const file = event.target.files[0];
    if (!file) return;

    const tier = getFileTier(file.size);
    if (tier === SIZE_TIERS.REJECTED) {
      showStorageWarning(warningDiv, 'File exceeds 100MB limit. Please use a smaller file.');
      fileInput.value = '';
      return;
    }

    try {
      const text = await readFile(file);
      textarea.value = text;
      if (tier === SIZE_TIERS.SMALL) {
        try {
          localStorage.setItem(textarea.id, text);
          clearStorageWarning(warningDiv);
        } catch (storageError) {
          showStorageWarning(
            warningDiv,
            storageError.name === 'QuotaExceededError'
              ? 'Storage quota exceeded. Content will not be persisted across sessions.'
              : `Storage error: ${storageError.message}`,
          );
        }
      } else {
        clearStorageWarning(warningDiv);
      }

      updateGutter(textarea, gutter);
      onContentChange(tier);
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

// ── Storage Warning Display ─────────────────────────

const showStorageWarning = (warningDiv, message) => {
  warningDiv.textContent = message;
  warningDiv.classList.remove('hidden');
};

const clearStorageWarning = (warningDiv) => {
  warningDiv.classList.add('hidden');
  warningDiv.textContent = '';
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
  const wrapper = document.createElement('div');
  wrapper.className = 'diff-node';
  wrapper.appendChild(div);
  return wrapper;
};

const renderDiffTree = (diffs, summaryEls, filter = null) => {
  const counts = countDiffs(diffs);
  summaryEls.additions.textContent = counts.additions;
  summaryEls.deletions.textContent = counts.deletions;
  summaryEls.modified.textContent = counts.modified;

  const treeEl = document.getElementById('diff-tree');
  treeEl.innerHTML = '';

  for (const node of diffs) {
    const el = renderDiffNode(node, true, filter);
    if (el) treeEl.appendChild(el);
  }
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
  const warningOne = document.getElementById('yaml-one-warning');
  const warningTwo = document.getElementById('yaml-two-warning');
  const fileOne = document.getElementById('file-one');
  const fileTwo = document.getElementById('file-two');
  const diffPlaceholder = document.getElementById('diff-placeholder');
  const diffSummary = document.getElementById('diff-summary');
  const diffLoader = document.getElementById('diff-loader');
  const diffLoaderText = document.getElementById('diff-loader-text');
  const diffTree = document.getElementById('diff-tree');
  const sizeWarningOne = document.getElementById('size-one-warning');
  const sizeWarningTwo = document.getElementById('size-two-warning');
  const diffRunBtn = document.getElementById('diff-run-btn');
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
  let diffInProgress = false;
  let diffPending = false;
  let requireExplicitDiff = false;

  // Placeholder helpers
  const showPlaceholder = () => {
    diffPlaceholder.classList.remove('hidden');
    diffSummary.classList.add('hidden');
    diffLoader.classList.add('hidden');
    diffTree.classList.add('hidden');
  };

  const hidePlaceholder = () => {
    diffPlaceholder.classList.add('hidden');
    diffSummary.classList.remove('hidden');
  };

  // Loader helpers
  const showLoader = (text) => {
    diffLoaderText.textContent = text;
    diffPlaceholder.classList.add('hidden');
    diffLoader.classList.remove('hidden');
    diffTree.classList.add('hidden');
  };

  const hideLoader = () => {
    diffLoader.classList.add('hidden');
    diffTree.classList.remove('hidden');
  };

  // Size tier helpers
  const updatePanelTier = (tier, byteSize, textarea, warningDiv) => {
    const MB = (byteSize / (1024 * 1024)).toFixed(1);
    if (tier === SIZE_TIERS.LARGE || tier === SIZE_TIERS.HUGE) {
      warningDiv.textContent = `${MB} MB — read-only mode, auto-diff disabled.`;
      warningDiv.classList.remove('hidden');
      textarea.readOnly = true;
    } else if (tier === SIZE_TIERS.MEDIUM) {
      warningDiv.textContent = `${MB} MB — localStorage disabled, content won't persist across sessions.`;
      warningDiv.classList.remove('hidden');
      textarea.readOnly = false;
    } else {
      warningDiv.classList.add('hidden');
      warningDiv.textContent = '';
      textarea.readOnly = false;
    }
  };

  const updateSizeTier = () => {
    const sizeOne = getContentByteSize(textAreaOne.value);
    const sizeTwo = getContentByteSize(textAreaTwo.value);
    const tierOne = getFileTier(sizeOne);
    const tierTwo = getFileTier(sizeTwo);

    updatePanelTier(tierOne, sizeOne, textAreaOne, sizeWarningOne);
    updatePanelTier(tierTwo, sizeTwo, textAreaTwo, sizeWarningTwo);

    const order = [SIZE_TIERS.SMALL, SIZE_TIERS.MEDIUM, SIZE_TIERS.LARGE, SIZE_TIERS.HUGE];
    const maxTier = order.indexOf(tierOne) >= order.indexOf(tierTwo) ? tierOne : tierTwo;

    if (maxTier === SIZE_TIERS.LARGE || maxTier === SIZE_TIERS.HUGE) {
      diffRunBtn.classList.remove('hidden');
      requireExplicitDiff = true;
    } else {
      diffRunBtn.classList.add('hidden');
      requireExplicitDiff = false;
    }
  };

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

  // Core diff (async with chunked processing)
  const runDiff = async () => {
    if (diffInProgress) {
      diffPending = true;
      return;
    }
    diffInProgress = true;
    diffPending = false;

    clearError(errorOne, gutterOne, textAreaOne);
    clearError(errorTwo, gutterTwo, textAreaTwo);

    const v1 = textAreaOne.value;
    const v2 = textAreaTwo.value;

    if (!v1.trim() && !v2.trim()) {
      showPlaceholder();
      diffInProgress = false;
      return;
    }

    hidePlaceholder();
    showLoader('Parsing YAML...');
    await yieldToUI();

    try {
      const keys = diff_init(v1, v2);

      let diffData;
      if (keys.length > 0) {
        diffData = [];
        for (let i = 0; i < keys.length; i++) {
          showLoader(`Computing diff (${i + 1}/${keys.length})...`);
          await yieldToUI();
          diffData.push(diff_key(keys[i]));
        }
      } else {
        showLoader('Computing diff...');
        await yieldToUI();
        diffData = diff_stored();
      }

      diff_cleanup();

      showLoader('Rendering...');
      await yieldToUI();

      lastDiffData = diffData;
      activeFilter = null;
      clearActiveFilterBtn();
      renderDiffTree(diffData, summaryEls);
      hideLoader();
    } catch (e) {
      diff_cleanup();
      hideLoader();
      showPlaceholder();
      if (e.message.includes('[YAML ONE]')) {
        showError(errorOne, gutterOne, textAreaOne, e.message, 'YAML ONE');
      }
      if (e.message.includes('[YAML TWO]')) {
        showError(errorTwo, gutterTwo, textAreaTwo, e.message, 'YAML TWO');
      }
    }

    diffInProgress = false;
    if (diffPending) {
      diffPending = false;
      runDiff();
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
      renderDiffTree(lastDiffData, summaryEls, activeFilter);
    }
  };

  filterBtns.additions.addEventListener('click', handleFilterClick);
  filterBtns.deletions.addEventListener('click', handleFilterClick);
  filterBtns.modified.addEventListener('click', handleFilterClick);

  // Input events
  const handleInput = (textarea, gutter, warningDiv) => {
    const tier = getFileTier(getContentByteSize(textarea.value));
    if (tier === SIZE_TIERS.SMALL) {
      try {
        localStorage.setItem(textarea.id, textarea.value);
        clearStorageWarning(warningDiv);
      } catch (storageError) {
        showStorageWarning(
          warningDiv,
          storageError.name === 'QuotaExceededError'
            ? 'Storage quota exceeded. Content will not be persisted across sessions.'
            : `Storage error: ${storageError.message}`,
        );
      }
    }
    updateGutter(textarea, gutter);
    updateSizeTier();
    if (!requireExplicitDiff) {
      debouncedDiff();
    }
  };

  textAreaOne.addEventListener('input', () => handleInput(textAreaOne, gutterOne, warningOne));
  textAreaTwo.addEventListener('input', () => handleInput(textAreaTwo, gutterTwo, warningTwo));

  // File uploads
  const onFileLoaded = () => {
    updateSizeTier();
    if (!requireExplicitDiff) {
      runDiff();
    }
  };
  setupFileUpload(fileOne, textAreaOne, gutterOne, warningOne, onFileLoaded);
  setupFileUpload(fileTwo, textAreaTwo, gutterTwo, warningTwo, onFileLoaded);

  // Explicit diff button for large files
  diffRunBtn.addEventListener('click', runDiff);

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
