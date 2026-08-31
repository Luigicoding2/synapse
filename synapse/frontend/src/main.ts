import './style.css';
// @ts-ignore
import ForceGraph from 'force-graph';
import * as App from '../wailsjs/go/main/App';

// UI State
let activeDirectoryPath = "";
let selectedNode: any = null;
let currentConfig: any = null;
let selectedNodeCode = "";
let graphInstance: any = null;

// Color Palette for Languages (Refined for Blue & Black aesthetic)
const langColors: Record<string, string> = {
    go: '#00a2ff',
    rust: '#f05a00',
    javascript: '#f7df1e',
    typescript: '#0066ff',
    python: '#38bdf8',
    cpp: '#0284c7',
    header: '#0ea5e9',
    java: '#0284c7',
    csharp: '#0369a1',
    folder: '#1e293b',
    unknown: '#475569'
};

// Simple Markdown Renderer (Modified pink to blue)
function parseMarkdown(text: string): string {
    let html = text;
    // Code blocks
    html = html.replace(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/g, (_, lang, code) => {
        const escaped = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
        return `<pre class="bg-black p-3 rounded-none border border-[#101424] text-[#38bdf8] overflow-x-auto my-2.5 font-mono select-text text-[9px]"><div class="text-[8px] text-[#0066ff] mb-1 font-semibold uppercase tracking-wider">${lang || 'code'}</div><code>${escaped}</code></pre>`;
    });
    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code class="bg-[#080a0f] border border-[#101424] text-[#38bdf8] px-1 rounded-none font-mono text-[10px]">$1</code>');
    // Strong text
    html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
    // Line breaks
    html = html.replace(/\n/g, '<br/>');
    return html;
}

// Byte size formatter
function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

// Custom Toast Notification System (Refined with sharp corners and blue borders)
function showToast(message: string, type: 'error' | 'success' | 'info' = 'info') {
    const container = document.getElementById('toast-container');
    if (!container) return;

    const toast = document.createElement('div');
    toast.className = `p-3 rounded-none border text-[10px] shadow-2xl transition-all duration-300 transform translate-y-4 opacity-0 pointer-events-auto flex items-center space-x-2.5 min-w-[280px] max-w-sm font-mono`;
    
    if (type === 'error') {
        toast.className += ' bg-[#090305] border-red-950/80 text-red-400';
    } else if (type === 'success') {
        toast.className += ' bg-[#030906] border-emerald-950/80 text-[#00b0ff]';
    } else {
        toast.className += ' bg-black border-[#101424] text-[#f1f5f9]';
    }

    const icon = type === 'error' 
        ? `<svg class="w-3.5 h-3.5 text-red-500 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path></svg>`
        : (type === 'success' 
            ? `<svg class="w-3.5 h-3.5 text-[#00b0ff] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>`
            : `<svg class="w-3.5 h-3.5 text-[#0066ff] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>`
        );

    toast.innerHTML = `
        ${icon}
        <div class="flex-1 leading-normal select-text">${message}</div>
        <button class="text-cyber-muted hover:text-white shrink-0 font-bold select-none cursor-pointer text-xs">&times;</button>
    `;

    toast.querySelector('button')!.addEventListener('click', () => {
        toast.classList.add('opacity-0', 'translate-y-4');
        setTimeout(() => toast.remove(), 300);
    });

    container.appendChild(toast);
    
    requestAnimationFrame(() => {
        setTimeout(() => {
            toast.classList.remove('opacity-0', 'translate-y-4');
        }, 10);
    });

    setTimeout(() => {
        if (toast.parentNode) {
            toast.classList.add('opacity-0', 'translate-y-4');
            setTimeout(() => toast.remove(), 300);
        }
    }, 5500);
}

// INITIALIZE APP
window.addEventListener('DOMContentLoaded', async () => {
    // Dom Elements
    const btnSelectDir = document.getElementById('btn-select-dir')!;
    const btnSelectDirLanding = document.getElementById('btn-select-dir-landing')!;
    const btnSettings = document.getElementById('btn-settings')!;
    const btnCloseSettings = document.getElementById('btn-close-settings')!;
    const btnSaveSettings = document.getElementById('btn-save-settings')!;
    const btnCloseNode = document.getElementById('btn-close-node')!;
    
    const landingOverlay = document.getElementById('landing-overlay')!;
    const loadingOverlay = document.getElementById('loading-overlay')!;
    const settingsModal = document.getElementById('settings-modal')!;
    
    const activePathDisplay = document.getElementById('active-path-display')!;
    const statRootPath = document.getElementById('stat-root-path')!;
    const statScannedTime = document.getElementById('stat-scanned-time')!;
    const statTotalFiles = document.getElementById('stat-total-files')!;
    const statTotalLines = document.getElementById('stat-total-lines')!;
    const languagesList = document.getElementById('languages-list')!;
    
    const panelDashboard = document.getElementById('panel-dashboard')!;
    const panelNode = document.getElementById('panel-node')!;
    
    const activeFileName = document.getElementById('active-file-name')!;
    const activeFilePath = document.getElementById('active-file-path')!;
    const activeFileLangDot = document.getElementById('active-file-lang-dot')!;
    const nodeStatLines = document.getElementById('node-stat-lines')!;
    const nodeStatSize = document.getElementById('node-stat-size')!;
    const nodeStatLang = document.getElementById('node-stat-lang')!;
    
    const tabCode = document.getElementById('tab-code')!;
    const tabChat = document.getElementById('tab-chat')!;
    const tabContentCode = document.getElementById('tab-content-code')!;
    const tabContentChat = document.getElementById('tab-content-chat')!;
    
    const codeContentPre = document.getElementById('code-content-pre')!;
    const chatMessages = document.getElementById('chat-messages')!;
    const chatInputText = document.getElementById('chat-input-text') as HTMLInputElement;
    const btnChatSend = document.getElementById('btn-chat-send')!;
    
    const settingsProvider = document.getElementById('settings-provider') as HTMLSelectElement;
    const settingsKey = document.getElementById('settings-key') as HTMLInputElement;
    const settingsUrl = document.getElementById('settings-url') as HTMLInputElement;
    const settingsModel = document.getElementById('settings-model') as HTMLInputElement;
    const settingsGroupKey = document.getElementById('settings-group-key')!;
    const settingsGroupUrl = document.getElementById('settings-group-url')!;

    // Load initial LLM configurations
    try {
        const configStr = await App.LoadConfig();
        currentConfig = JSON.parse(configStr);
        populateSettingsInputs();
    } catch (e) {
        console.error("Failed loading configurations", e);
    }

    // Toggle settings inputs based on provider choice
    settingsProvider.addEventListener('change', () => {
        updateSettingsUI();
    });

    function updateSettingsUI() {
        const provider = settingsProvider.value;
        if (provider === 'gemini') {
            settingsGroupKey.classList.remove('hidden');
            settingsGroupUrl.classList.add('hidden');
        } else if (provider === 'ollama') {
            settingsGroupKey.classList.add('hidden');
            settingsGroupUrl.classList.remove('hidden');
            settingsUrl.placeholder = "http://localhost:11434";
        } else { // Custom
            settingsGroupKey.classList.remove('hidden');
            settingsGroupUrl.classList.remove('hidden');
            settingsUrl.placeholder = "https://api.omniroute.com/v1";
        }
    }

    function populateSettingsInputs() {
        if (!currentConfig) return;
        settingsProvider.value = currentConfig.provider || 'ollama';
        settingsKey.value = currentConfig.apiKey || '';
        settingsUrl.value = currentConfig.baseUrl || '';
        settingsModel.value = currentConfig.modelName || '';
        updateSettingsUI();
    }

    // Action: Open settings dialog
    btnSettings.addEventListener('click', () => {
        populateSettingsInputs();
        settingsModal.classList.remove('hidden');
    });

    btnCloseSettings.addEventListener('click', () => {
        settingsModal.classList.add('hidden');
    });

    // Action: Save configurations (Using toast notifications instead of alerts)
    btnSaveSettings.addEventListener('click', async () => {
        const updatedConfig = {
            provider: settingsProvider.value,
            apiKey: settingsKey.value,
            baseUrl: settingsUrl.value,
            modelName: settingsModel.value,
        };
        try {
            await App.SaveConfig(JSON.stringify(updatedConfig));
            currentConfig = updatedConfig;
            settingsModal.classList.add('hidden');
            showToast("Configurations saved successfully!", "success");
        } catch (e) {
            showToast("Error saving configurations: " + e, "error");
        }
    });

    // Action: Open Folder picker
    const triggerFolderPicker = async () => {
        try {
            const dir = await App.SelectDirectory();
            if (dir) {
                activeDirectoryPath = dir;
                await scanAndRender();
            }
        } catch (e) {
            console.error("Error picking directory", e);
        }
    };

    btnSelectDir.addEventListener('click', triggerFolderPicker);
    btnSelectDirLanding.addEventListener('click', triggerFolderPicker);

    // CORE: Scan directory and render force graph
    async function scanAndRender() {
        loadingOverlay.classList.remove('hidden');
        try {
            const resultJson = await App.ScanDirectory(activeDirectoryPath);
            const graphData = JSON.parse(resultJson);
            
            loadingOverlay.classList.add('hidden');
            landingOverlay.classList.add('hidden');

            activePathDisplay.innerText = activeDirectoryPath;
            statRootPath.innerText = activeDirectoryPath;
            statScannedTime.innerText = new Date().toLocaleTimeString();

            calculateStats(graphData);
            renderGraph(graphData);
            showToast("Codebase indexed successfully!", "success");
        } catch (e) {
            loadingOverlay.classList.add('hidden');
            showToast("Error scanning codebase: " + e, "error");
        }
    }

    // Calculate codebase metrics
    function calculateStats(data: any) {
        const files = data.nodes.filter((n: any) => n.type === 'file');
        statTotalFiles.innerText = files.length.toString();

        const totalLines = files.reduce((acc: number, f: any) => acc + f.lines, 0);
        statTotalLines.innerText = totalLines.toLocaleString();

        // Calculate language counts
        const counts: Record<string, { files: number, lines: number }> = {};
        files.forEach((f: any) => {
            const lang = f.language || 'unknown';
            if (!counts[lang]) {
                counts[lang] = { files: 0, lines: 0 };
            }
            counts[lang].files += 1;
            counts[lang].lines += f.lines;
        });

        // Sort languages by file count
        const sortedLangs = Object.entries(counts).sort((a, b) => b[1].files - a[1].files);

        languagesList.innerHTML = '';
        sortedLangs.forEach(([lang, info]) => {
            const color = langColors[lang] || langColors.unknown;
            const percentage = totalLines > 0 ? Math.round((info.lines / totalLines) * 100) : 0;
            
            const div = document.createElement('div');
            div.className = "flex flex-col space-y-1";
            div.innerHTML = `
                <div class="flex items-center justify-between text-[10px] font-mono">
                    <span class="flex items-center space-x-1.5 capitalize">
                        <span class="w-1.5 h-1.5 rounded-none" style="background-color: ${color}"></span>
                        <span class="text-white">${lang}</span>
                        <span class="text-cyber-muted text-[9px]">(${info.files} files)</span>
                    </span>
                    <span class="text-white font-semibold">${percentage}%</span>
                </div>
                <div class="w-full bg-[#050609] h-[3px] border border-[#101424] rounded-none overflow-hidden">
                    <div class="h-full rounded-none" style="background-color: ${color}; width: ${percentage}%"></div>
                </div>
            `;
            languagesList.appendChild(div);
        });
    }

    // RENDER FORCE GRAPH ON CANVAS
    function renderGraph(data: any) {
        const container = document.getElementById('graph-container')!;
        
        // Remove old graph canvas
        container.innerHTML = '';

        // Reset selected node state
        selectedNode = null;
        panelDashboard.classList.remove('hidden');
        panelNode.classList.add('hidden');

        // Create Force Graph Instance (showNavInfo removed to prevent 2D TypeError crash)
        graphInstance = ForceGraph()(container)
            .graphData(data)
            .backgroundColor('#000000')
            .nodeId('id')
            .nodeVal((node: any) => {
                if (node.type === 'dir') return 3;
                return node.size ? Math.max(3, Math.min(10, Math.log10(node.size) * 1.5)) : 4;
            })
            // CUSTOM CANVAS NODE DRAWING (Refined for high-contrast scaling labels and blue-black theme)
            .nodeCanvasObject((node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
                const label = node.name;
                const size = node.size ? Math.max(3, Math.min(10, Math.log10(node.size) * 1.5)) : 4;
                const r = node.type === 'dir' ? 3.5 : size;

                ctx.beginPath();
                ctx.arc(node.x, node.y, r, 0, 2 * Math.PI, false);

                const color = node.type === 'dir' ? langColors.folder : (langColors[node.language] || langColors.unknown);
                ctx.fillStyle = color;
                ctx.fill();

                // Draw outer borders for active selections or folders (Glow halo is cobalt-blue)
                if (selectedNode && selectedNode.id === node.id) {
                    ctx.strokeStyle = '#00b0ff';
                    ctx.lineWidth = 2.0 / globalScale;
                    ctx.stroke();
                } else if (node.type === 'dir') {
                    ctx.strokeStyle = '#1e293b';
                    ctx.lineWidth = 0.8 / globalScale;
                    ctx.stroke();
                } else {
                    ctx.strokeStyle = '#000000';
                    ctx.lineWidth = 0.5 / globalScale;
                    ctx.stroke();
                }

                // Node names displayed underneath circle in canvas-relative units (scales with zoom)
                if (globalScale > 0.6) {
                    const fontSize = 3.2; // canvas-relative size
                    ctx.font = `${fontSize}px ui-monospace, SFMono-Regular, Consolas, monospace`;
                    ctx.textAlign = 'center';
                    ctx.textBaseline = 'top';

                    const textWidth = ctx.measureText(label).width;
                    
                    // Draw a crisp dark background badge behind the text for premium readability
                    ctx.fillStyle = 'rgba(0, 0, 0, 0.75)';
                    ctx.fillRect(node.x - textWidth/2 - 0.6, node.y + r + 1.2, textWidth + 1.2, fontSize + 0.8);
                    
                    // Draw a thin border on the badge if selected
                    if (selectedNode && selectedNode.id === node.id) {
                        ctx.strokeStyle = 'rgba(0, 176, 255, 0.3)';
                        ctx.lineWidth = 0.2;
                        ctx.strokeRect(node.x - textWidth/2 - 0.6, node.y + r + 1.2, textWidth + 1.2, fontSize + 0.8);
                    }

                    ctx.fillStyle = node.type === 'dir' ? '#64748b' : '#f1f5f9';
                    ctx.fillText(label, node.x, node.y + r + 1.6);
                }
            })
            // DYNAMIC LINK COLORING (Glowing Cyan active, subtle transparent blue default)
            .linkColor((link: any) => {
                if (selectedNode) {
                    if (link.source.id === selectedNode.id || link.target.id === selectedNode.id) {
                        return '#00b0ff'; // Ice Cyan glowing active path
                    }
                    return 'rgba(0, 102, 255, 0.03)'; // Almost invisible non-active paths
                }
                return 'rgba(0, 102, 255, 0.25)'; // Subtle cobalt-cyan lines showing graph map by default
            })
            .linkWidth((link: any) => {
                if (selectedNode && (link.source.id === selectedNode.id || link.target.id === selectedNode.id)) {
                    return 1.5;
                }
                return 0.75;
            })
            // ANIMATED FLOW PARTICLES ALONG DEPENDENCIES
            .linkDirectionalParticles((link: any) => {
                if (selectedNode && (link.source.id === selectedNode.id || link.target.id === selectedNode.id)) {
                    return 2;
                }
                return 0;
            })
            .linkDirectionalParticleWidth(1.5)
            .linkDirectionalParticleSpeed(0.005)
            
            // CLICK EVENTS
            .onNodeClick((node: any) => {
                // Focus camera on clicked node
                graphInstance.centerAt(node.x, node.y, 400);
                graphInstance.zoom(2.5, 400);

                if (node.type === 'dir') {
                    return;
                }

                selectedNode = node;
                
                // Show File Panel
                panelDashboard.classList.add('hidden');
                panelNode.classList.remove('hidden');

                // Fill file metadata info
                activeFileName.innerText = node.name;
                activeFilePath.innerText = node.id;
                activeFileLangDot.style.backgroundColor = langColors[node.language] || langColors.unknown;
                nodeStatLines.innerText = node.lines.toLocaleString();
                nodeStatSize.innerText = formatBytes(node.size);
                nodeStatLang.innerText = node.language || 'unknown';

                // Trigger file loading
                loadActiveFileCode();
            })
            .onBackgroundClick(() => {
                selectedNode = null;
                panelDashboard.classList.remove('hidden');
                panelNode.classList.add('hidden');
            });
    }

    // Load file content for Code View tab
    async function loadActiveFileCode() {
        if (!selectedNode) return;
        
        codeContentPre.innerText = "Loading file content...";
        selectedNodeCode = "";
        
        // Reset tab view to Code
        showTab('code');

        // Reset chat history for this node
        chatMessages.innerHTML = `
            <div class="bg-[#050609] p-2.5 rounded-none border border-[#101424] leading-relaxed">
                <span class="text-[#00b0ff] font-bold block text-[9px] mb-1">SYNAPSE AI:</span>
                I've indexed <code class="bg-[#000000] border border-[#101424] px-1 rounded-none text-white">${selectedNode.name}</code>. You can ask me details about its variables, imports, or request refactoring patterns.
            </div>
        `;

        try {
            const code = await App.GetFileContent(selectedNode.id);
            selectedNodeCode = code;
            codeContentPre.innerText = code;
        } catch (e) {
            codeContentPre.innerText = "Failed loading code file: " + e;
        }
    }

    // CLOSE NODE VIEW
    btnCloseNode.addEventListener('click', () => {
        selectedNode = null;
        panelDashboard.classList.remove('hidden');
        panelNode.classList.add('hidden');
    });

    // PANEL TABS SELECTION (Aligned with sharp border rules)
    tabCode.addEventListener('click', () => showTab('code'));
    tabChat.addEventListener('click', () => showTab('chat'));

    function showTab(tab: 'code' | 'chat') {
        if (tab === 'code') {
            tabCode.className = "flex-1 py-2 text-center text-white border-b border-[#0066ff] bg-[#050609] focus:outline-none rounded-none";
            tabChat.className = "flex-1 py-2 text-center text-cyber-muted hover:text-white border-b border-transparent hover:bg-[#030406] focus:outline-none rounded-none";
            tabContentCode.classList.remove('hidden');
            tabContentChat.classList.add('hidden');
        } else {
            tabChat.className = "flex-1 py-2 text-center text-white border-b border-[#0066ff] bg-[#050609] focus:outline-none rounded-none";
            tabCode.className = "flex-1 py-2 text-center text-cyber-muted hover:text-white border-b border-transparent hover:bg-[#030406] focus:outline-none rounded-none";
            tabContentChat.classList.remove('hidden');
            tabContentCode.classList.add('hidden');
        }
    }

    // CHAT SYSTEM CONTROLLER
    const sendChatMessage = async () => {
        const text = chatInputText.value.trim();
        if (!text || !selectedNode) return;

        // Display user query
        appendMessage('USER', text);
        chatInputText.value = '';

        // Typing / Thinking Indicator
        const thinkingDiv = appendMessage('AI', 'Thinking...');
        
        // Disable input during prompt duration
        chatInputText.disabled = true;
        btnChatSend.setAttribute('disabled', 'true');

        try {
            const aiReply = await App.AskAI(text, selectedNode.id, selectedNodeCode);
            thinkingDiv.innerHTML = `
                <span class="text-[#00b0ff] font-bold block text-[9px] mb-1">SYNAPSE AI:</span>
                ${parseMarkdown(aiReply)}
            `;
        } catch (e) {
            thinkingDiv.innerHTML = `
                <span class="text-red-500 font-bold block text-[9px] mb-1">ERROR:</span>
                Failed generating response: ${e}
            `;
        } finally {
            chatInputText.disabled = false;
            btnChatSend.removeAttribute('disabled');
            chatInputText.focus();
            
            // Scroll to bottom
            chatMessages.scrollTop = chatMessages.scrollHeight;
        }
    };

    function appendMessage(sender: 'USER' | 'AI', msg: string): HTMLDivElement {
        const msgDiv = document.createElement('div');
        msgDiv.className = "p-2.5 rounded-none border leading-relaxed";
        
        if (sender === 'USER') {
            msgDiv.className += " bg-black border-[#101424]";
            msgDiv.innerHTML = `
                <span class="text-cyber-muted font-bold block text-[9px] mb-1">USER:</span>
                ${msg.replace(/\n/g, '<br/>')}
            `;
        } else {
            msgDiv.className += " bg-[#050609] border-[#101424]";
            msgDiv.innerHTML = `
                <span class="text-[#00b0ff] font-bold block text-[9px] mb-1">SYNAPSE AI:</span>
                ${parseMarkdown(msg)}
            `;
        }

        chatMessages.appendChild(msgDiv);
        chatMessages.scrollTop = chatMessages.scrollHeight;
        return msgDiv;
    }

    btnChatSend.addEventListener('click', sendChatMessage);
    chatInputText.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            sendChatMessage();
        }
    });
});
