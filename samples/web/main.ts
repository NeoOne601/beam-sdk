// samples/web/main.ts
// Beam Verify Web Sample — Main entry point.
// Connects UI screens to camera, quality gate simulation, and backend verification.

// Screen navigation
function showScreen(id: string) {
    document.querySelectorAll('.screen').forEach(s => s.classList.remove('active'));
    document.getElementById(id)?.classList.add('active');
}

// Settings persistence
function loadSettings() {
    return {
        backendUrl: localStorage.getItem('beam_backend_url') || 'http://localhost:8080',
        pqcSigning: localStorage.getItem('beam_pqc') !== 'false',
        includeRawMrz: localStorage.getItem('beam_mrz') === 'true',
    };
}

function saveSettings() {
    const url = (document.getElementById('settingBackendUrl') as HTMLInputElement).value;
    const pqc = (document.getElementById('settingPqc') as HTMLInputElement).checked;
    const mrz = (document.getElementById('settingMrz') as HTMLInputElement).checked;
    localStorage.setItem('beam_backend_url', url);
    localStorage.setItem('beam_pqc', String(pqc));
    localStorage.setItem('beam_mrz', String(mrz));
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

// Simulated scan flow
async function runScanFlow() {
    showScreen('scan');
    const video = document.getElementById('cameraPreview') as HTMLVideoElement;
    const gateLabel = document.getElementById('gateLabel')!;
    const frameCount = document.getElementById('frameCount')!;
    const qualityFill = document.querySelector('.quality-fill') as HTMLElement;

    const stream = await startCamera();
    if (!stream) { showScreen('landing'); return; }
    video.srcObject = stream;

    const gates = ['BlurCheck', 'ExposureCheck', 'MotionCheck', 'BoundaryCheck', 'Accepted'];

    for (let i = 0; i < gates.length; i++) {
        await delay(800);
        gateLabel.textContent = `Gate: ${gates[i]}`;
        qualityFill.style.width = `${((i + 1) / gates.length) * 100}%`;
        if (gates[i] === 'Accepted') {
            frameCount.textContent = 'Quality frames: 3 / 3';
        }
    }

    await delay(1000);
    gateLabel.textContent = 'Running ONNX inference...';
    await delay(1000);
    gateLabel.textContent = 'Signing with ML-DSA Level 3...';
    await delay(1000);

    stopCamera(stream);
    showResult();
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

// Helpers
function delay(ms: number) { return new Promise(r => setTimeout(r, ms)); }

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
