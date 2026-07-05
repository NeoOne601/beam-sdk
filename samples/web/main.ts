// samples/web/main.ts
// Ajna Verify Web Sample — Main entry point.
// Connects UI screens to camera, quality gate, and backend verification.
// Uses the real AjnaScanner SDK wrapper for WASM inference.

import { AjnaScanner, ScanResult } from '../../platform/web/AjnaScanner.js';

// Screen navigation
function showScreen(id: string) {
    document.querySelectorAll('.screen').forEach(s => s.classList.remove('active'));
    document.getElementById(id)?.classList.add('active');
}

// Settings persistence
function loadSettings() {
    return {
        backendUrl: localStorage.getItem('ajna_backend_url') || 'http://localhost:8080',
        pqcSigning: localStorage.getItem('ajna_pqc') !== 'false',
        includeRawMrz: localStorage.getItem('ajna_mrz') === 'true',
    };
}

function saveSettings() {
    const url = (document.getElementById('settingBackendUrl') as HTMLInputElement).value;
    const pqc = (document.getElementById('settingPqc') as HTMLInputElement).checked;
    const mrz = (document.getElementById('settingMrz') as HTMLInputElement).checked;
    localStorage.setItem('ajna_backend_url', url);
    localStorage.setItem('ajna_pqc', String(pqc));
    localStorage.setItem('ajna_mrz', String(mrz));
    showScreen('landing');
}

// Camera access
async function startCamera(): Promise<MediaStream | null> {
    try {
        return await navigator.mediaDevices.getUserMedia({
            video: { facingMode: 'environment', width: { ideal: 1920 }, height: { ideal: 1080 } },
        });
    } catch (e) {
        console.error('Camera access denied:', e);
        alert('Camera permission is required for document scanning.');
        return null;
    }
}

function stopCamera(stream: MediaStream | null) {
    stream?.getTracks().forEach(t => t.stop());
    const video = document.getElementById('cameraPreview') as HTMLVideoElement;
    video.srcObject = null;
}

// Real scan flow using AjnaScanner SDK
async function runScanFlow() {
    showScreen('scan');
    const video = document.getElementById('cameraPreview') as HTMLVideoElement;
    const gateLabel = document.getElementById('gateLabel')!;
    const qualityFill = document.querySelector('.quality-fill') as HTMLElement;

    const stream = await startCamera();
    if (!stream) { showScreen('landing'); return; }
    video.srcObject = stream;

    const settings = loadSettings();
    const scanner = new AjnaScanner();

    try {
        await scanner.configure({
            apiKey: '',
            backendUrl: settings.backendUrl,
            modelPath: '/ajna_sdk.onnx',
            enablePqcSigning: settings.pqcSigning,
        });
    } catch (e) {
        console.error('[AjnaSample] Failed to configure scanner:', e);
        stopCamera(stream);
        showScreen('landing');
        return;
    }

    const canvas = document.createElement('canvas');
    canvas.width = video.videoWidth || 1280;
    canvas.height = video.videoHeight || 720;
    const ctx = canvas.getContext('2d')!;

    const processLoop = async () => {
        ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
        const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
        const report = await scanner.processFrame(imageData);
        gateLabel.textContent = `Gate: ${report.gateReached}`;

        // Update quality fill based on gate progress
        const gateMap: Record<string, number> = {
            'Idle': 0, 'Scanning': 25, 'Inferring': 50, 'Complete': 100, 'Failed': 0
        };
        qualityFill.style.width = `${gateMap[report.gateReached] ?? 0}%`;

        if (report.gateReached === 'Complete') {
            const result = await scanner.getScanResult();
            stopCamera(stream);
            if (result) showResultFromSdk(result);
            return;
        }
        if (report.gateReached !== 'Failed') {
            requestAnimationFrame(() => processLoop());
        }
    };
    requestAnimationFrame(() => processLoop());
}

// Display result from real SDK ScanResult
function showResultFromSdk(scanResult: ScanResult) {
    const fieldList = document.getElementById('fieldList')!;
    fieldList.innerHTML = scanResult.fields.map(f => `
        <div class="field-item">
            <div class="field-key">${f.key.replace(/_/g, ' ')}</div>
            <div class="field-value-row">
                <span class="field-value">${f.value}</span>
                <span class="field-conf">${Math.round(f.confidence * 100)}%</span>
            </div>
        </div>
    `).join('');

    document.getElementById('docType')!.textContent = scanResult.documentType.toUpperCase();
    document.getElementById('country')!.textContent = scanResult.issuingCountry;
    const confPct = Math.round(scanResult.confidence * 100);
    document.getElementById('confidenceText')!.textContent = `${confPct}%`;

    const arc = document.getElementById('gaugeArc')!;
    arc.setAttribute('stroke-dasharray', `${confPct}, 100`);

    showScreen('result');
}

function showResult() {
    const fields = [
        { key: 'surname', value: 'SMITH', confidence: 0.98 },
        { key: 'given_names', value: 'JOHN MICHAEL', confidence: 0.96 },
        { key: 'date_of_birth', value: '1990-05-15', confidence: 0.99 },
        { key: 'document_number', value: 'C01X00T47', confidence: 0.97 },
        { key: 'expiry_date', value: '2030-05-14', confidence: 0.95 },
    ];

    const fieldList = document.getElementById('fieldList')!;
    fieldList.innerHTML = fields.map(f => `
        <div class="field-item">
            <div class="field-key">${f.key.replace(/_/g, ' ')}</div>
            <div class="field-value-row">
                <span class="field-value">${f.value}</span>
                <span class="field-conf">${Math.round(f.confidence * 100)}%</span>
            </div>
        </div>
    `).join('');

    document.getElementById('docType')!.textContent = 'PASSPORT';
    document.getElementById('country')!.textContent = 'USA';
    document.getElementById('confidenceText')!.textContent = '97%';

    const arc = document.getElementById('gaugeArc')!;
    arc.setAttribute('stroke-dasharray', '97, 100');

    showScreen('result');
}

// Backend verification
async function verifyWithBackend() {
    const btn = document.getElementById('btnVerify') as HTMLButtonElement;
    btn.disabled = true;
    btn.textContent = 'Verifying...';

    try {
        const settings = loadSettings();
        const sessionId = crypto.randomUUID();

        // Step 1: Get nonce
        const nonceResp = await fetch(`${settings.backendUrl}/v1/nonce`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ session_id: sessionId }),
        });
        const nonceData = await nonceResp.json();

        // Step 2: Verify
        const verifyResp = await fetch(`${settings.backendUrl}/v1/verify`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                session_id: sessionId,
                nonce: nonceData.nonce,
                scan_result: {
                    fields: [],
                    document_type: 'passport',
                    issuing_country: 'USA',
                    confidence: 0.97,
                    pqc_signature: '',
                    pqc_public_key: '',
                },
            }),
        });

        if (verifyResp.ok) {
            btn.textContent = '✓ Verified';
            btn.style.background = 'var(--teal)';
        } else {
            btn.textContent = 'Verification Failed';
            btn.style.background = 'var(--error)';
        }
    } catch (e) {
        btn.textContent = `Error: ${(e as Error).message}`;
        btn.style.background = 'var(--error)';
    }

    setTimeout(() => {
        btn.disabled = false;
        btn.textContent = 'Verify with Backend';
        btn.style.background = '';
    }, 3000);
}

// Event binding
document.addEventListener('DOMContentLoaded', () => {
    // Load settings into form
    const s = loadSettings();
    (document.getElementById('settingBackendUrl') as HTMLInputElement).value = s.backendUrl;
    (document.getElementById('settingPqc') as HTMLInputElement).checked = s.pqcSigning;
    (document.getElementById('settingMrz') as HTMLInputElement).checked = s.includeRawMrz;

    // Navigation
    document.getElementById('btnStartScan')!.addEventListener('click', runScanFlow);
    document.getElementById('btnSettings')!.addEventListener('click', () => showScreen('settings'));
    document.getElementById('btnCancelScan')!.addEventListener('click', () => {
        stopCamera(null);
        showScreen('landing');
    });
    document.getElementById('btnNewScan')!.addEventListener('click', () => showScreen('landing'));
    document.getElementById('btnVerify')!.addEventListener('click', verifyWithBackend);
    document.getElementById('btnSaveSettings')!.addEventListener('click', saveSettings);
    document.getElementById('btnBackFromSettings')!.addEventListener('click', () => showScreen('landing'));
});
