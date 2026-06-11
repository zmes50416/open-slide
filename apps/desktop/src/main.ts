import { invoke } from '@tauri-apps/api/core';

interface DetectedAgent {
  id: string;
  name: string;
  path: string;
}

const SLIDES_URL = import.meta.env.DEV ? 'http://localhost:5173/' : '/slides/index.html';

const folderPath = document.getElementById('folder-path') as HTMLSpanElement;
const folderBox = document.getElementById('folder') as HTMLDivElement;
const agentControls = document.getElementById('agent-controls') as HTMLDivElement;
const agentSelect = document.getElementById('agent-select') as HTMLSelectElement;
const launchButton = document.getElementById('launch') as HTMLButtonElement;
const noAgents = document.getElementById('no-agents') as HTMLSpanElement;
const rescanButton = document.getElementById('rescan') as HTMLButtonElement;
const status = document.getElementById('status') as HTMLSpanElement;
const iframe = document.getElementById('slides') as HTMLIFrameElement;
const veil = document.getElementById('veil') as HTMLDivElement;
const veilText = document.getElementById('veil-text') as HTMLParagraphElement;

let statusTimer: ReturnType<typeof setTimeout> | undefined;

function showStatus(message: string, kind: 'ok' | 'error') {
  clearTimeout(statusTimer);
  status.textContent = message;
  status.className = `status ${kind}`;
  status.title = message;
  statusTimer = setTimeout(() => {
    status.textContent = '';
    status.title = '';
  }, 6000);
}

async function loadFolder() {
  try {
    const folder = await invoke<string>('get_project_folder');
    folderPath.textContent = folder;
    folderBox.title = folder;
  } catch (error) {
    folderPath.textContent = 'unavailable';
    folderBox.title = String(error);
  }
}

async function scanAgents() {
  rescanButton.classList.add('spinning');
  try {
    const agents = await invoke<DetectedAgent[]>('detect_agents');
    const previous = agentSelect.value;
    agentSelect.innerHTML = '';
    for (const agent of agents) {
      const option = document.createElement('option');
      option.value = agent.id;
      option.textContent = agent.name;
      option.title = agent.path;
      agentSelect.append(option);
    }
    if (agents.some((agent) => agent.id === previous)) {
      agentSelect.value = previous;
    }
    agentControls.hidden = agents.length === 0;
    noAgents.hidden = agents.length > 0;
  } catch (error) {
    showStatus(String(error), 'error');
  } finally {
    rescanButton.classList.remove('spinning');
  }
}

async function launchAgent() {
  const agentId = agentSelect.value;
  if (!agentId) return;
  launchButton.disabled = true;
  try {
    await invoke('launch_agent', { agentId });
    const name = agentSelect.selectedOptions[0]?.textContent ?? agentId;
    showStatus(`${name} opened in a terminal`, 'ok');
  } catch (error) {
    showStatus(String(error), 'error');
  } finally {
    launchButton.disabled = false;
  }
}

async function slidesReachable(): Promise<boolean> {
  try {
    await fetch(SLIDES_URL, { mode: 'no-cors', cache: 'no-store' });
    return true;
  } catch {
    return false;
  }
}

async function mountSlides() {
  if (import.meta.env.DEV) {
    let attempts = 0;
    while (!(await slidesReachable())) {
      attempts += 1;
      if (attempts === 20) {
        veilText.textContent = 'still waiting for the dev server on :5173';
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  iframe.addEventListener(
    'load',
    () => {
      iframe.classList.add('ready');
      veil.classList.add('hidden');
    },
    { once: true },
  );
  iframe.src = SLIDES_URL;
}

launchButton.addEventListener('click', launchAgent);
rescanButton.addEventListener('click', scanAgents);

loadFolder();
scanAgents();
mountSlides();
