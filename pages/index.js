// CSS Assets
import './style.css';

import { diff } from '../pkg';

const ready = (fn) => {
  if (document.readyState != 'loading') {
    fn();
  } else {
    document.addEventListener('DOMContentLoaded', fn);
  }
};

const supported = (() => {
  try {
    if (typeof WebAssembly === "object" && typeof WebAssembly.instantiate === "function") {
      const module = new WebAssembly.Module(Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00));
      if (module instanceof WebAssembly.Module)
        return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
    }
  } catch (e) {
    console.error(e);
  }
  return false;
});

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
  const savedTextAreaOne = localStorage.getItem('text-area-one');
  const savedTextAreaTwo = localStorage.getItem('text-area-two');
  const yamlOneError = document.getElementById('yaml-one-error');
  const yamlTwoError = document.getElementById('yaml-two-error');
  textAreaOne.value = savedTextAreaOne || '';
  textAreaTwo.value = savedTextAreaTwo || '';
  textAreaOne.addEventListener('input', (event) => {
    localStorage.setItem('text-area-one', event.target.value);
    yamlOneError.classList.add('hidden');
  });
  textAreaTwo.addEventListener('input', (event) => {
    localStorage.setItem('text-area-two', event.target.value);
    yamlTwoError.classList.add('hidden');
  });
  const compareButton = document.getElementById('compare-button');
  const diffContainer = document.getElementById('diff-container');
  const diffAdditions = document.getElementById('diff-additions');
  const diffDeletions = document.getElementById('diff-deletions');
  const diffConflicts = document.getElementById('diff-conflicts');
  compareButton.addEventListener('click', () => {
    try {
      const diffData = diff(textAreaOne.value, textAreaTwo.value);
      diffContainer.classList.remove('hidden');
      console.log(diffData);
    } catch (e) {
      console.error(e);
      if (e.message.includes('[YAML ONE]')) {
        yamlOneError.classList.remove('hidden');
        yamlOneError.innerText = e.message.replace('[YAML ONE]', '');
      } else if (e.message.includes('[YAML TWO]')) {
        yamlTwoError.classList.remove('hidden');
        yamlTwoError.innerText = e.message.replace('[YAML TWO]', '');
      }
    }
  });
});
