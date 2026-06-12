import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface DetectedAgent {
  id: string;
  name: string;
  path: string;
}

interface ChosenFolder {
  path: string;
  isProject: boolean;
}

const folderPath = document.getElementById('folder-path') as HTMLSpanElement;
const folderBox = document.getElementById('folder') as HTMLDivElement;
const changeFolderButton = document.getElementById('change-folder') as HTMLButtonElement;
const agentControls = document.getElementById('agent-controls') as HTMLDivElement;
const agentSelect = document.getElementById('agent-select') as HTMLSelectElement;
const launchButton = document.getElementById('launch') as HTMLButtonElement;
const noAgents = document.getElementById('no-agents') as HTMLSpanElement;
const rescanButton = document.getElementById('rescan') as HTMLButtonElement;
const status = document.getElementById('status') as HTMLSpanElement;
const iframe = document.getElementById('slides') as HTMLIFrameElement;
const veil = document.getElementById('veil') as HTMLDivElement;
const veilText = document.getElementById('veil-text') as HTMLParagraphElement;
const veilDetail = document.getElementById('veil-detail') as HTMLParagraphElement;
const firstRun = document.getElementById('first-run') as HTMLElement;
const chooseFolderButton = document.getElementById('choose-folder') as HTMLButtonElement;
const firstRunError = document.getElementById('first-run-error') as HTMLParagraphElement;

let statusTimer: ReturnType<typeof setTimeout> | undefined;
let starting = false;

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

function setFolder(path: string | null) {
  folderPath.textContent = path ?? '—';
  folderBox.title = path ?? '';
  changeFolderButton.hidden = path === null;
}

function showVeil(text: string) {
  firstRun.hidden = true;
  veil.classList.remove('hidden');
  veilText.textContent = text;
  veilDetail.textContent = '';
}

function hideVeil() {
  veil.classList.add('hidden');
}

function showFirstRun(error?: string) {
  hideVeil();
  iframe.classList.remove('ready');
  iframe.removeAttribute('src');
  firstRun.hidden = false;
  firstRunError.hidden = !error;
  firstRunError.textContent = error ?? '';
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

async function devServerReachable(port: number): Promise<boolean> {
  try {
    await fetch(`http://localhost:${port}/`, { mode: 'no-cors', cache: 'no-store' });
    return true;
  } catch {
    return false;
  }
}

async function startProject() {
  if (starting) return;
  starting = true;
  try {
    showVeil('starting the slide dev server');
    const port = await invoke<number>('start_dev_server');
    let attempts = 0;
    while (!(await devServerReachable(port))) {
      attempts += 1;
      if (attempts > 240) {
        throw new Error(`The dev server did not come up on port ${port}.`);
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    iframe.classList.remove('ready');
    iframe.addEventListener(
      'load',
      () => {
        iframe.classList.add('ready');
        hideVeil();
      },
      { once: true },
    );
    iframe.src = `http://localhost:${port}/`;
  } catch (error) {
    showFirstRun(String(error));
  } finally {
    starting = false;
  }
}

async function chooseFolder() {
  chooseFolderButton.disabled = true;
  changeFolderButton.disabled = true;
  try {
    const chosen = await invoke<ChosenFolder | null>('choose_project_folder');
    if (!chosen) return;
    setFolder(chosen.path);
    if (!chosen.isProject) {
      showVeil('initializing your slide project');
      await invoke('init_project');
      showStatus('starter project created', 'ok');
    }
    await startProject();
  } catch (error) {
    showFirstRun(String(error));
  } finally {
    chooseFolderButton.disabled = false;
    changeFolderButton.disabled = false;
  }
}

async function boot() {
  listen<string>('setup-log', (event) => {
    veilDetail.textContent = event.payload;
  });
  listen<string>('dev-log', (event) => {
    if (!veil.classList.contains('hidden')) {
      veilDetail.textContent = event.payload;
    }
  });

  scanAgents();

  const folder = await invoke<string | null>('get_project_folder');
  setFolder(folder);
  if (folder === null) {
    showFirstRun();
    return;
  }
  await startProject();
}

launchButton.addEventListener('click', launchAgent);
rescanButton.addEventListener('click', scanAgents);
chooseFolderButton.addEventListener('click', chooseFolder);
changeFolderButton.addEventListener('click', chooseFolder);

boot();
