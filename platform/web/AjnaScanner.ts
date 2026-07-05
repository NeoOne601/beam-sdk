// platform/web/AjnaScanner.ts
// High-level TypeScript SDK wrapper for the Ajna WASM module.
// Encapsulates all WASM memory management (_malloc, _free, HEAPU8)
// behind a clean async API.

export interface AjnaConfig {
  apiKey: string;
  backendUrl: string;
  modelPath: string;
  enablePqcSigning?: boolean;
}

export interface DocumentField {
  key: string;
  value: string;
  confidence: number;
}

export interface ScanResult {
  fields: DocumentField[];
  documentType: string;
  issuingCountry: string;
  confidence: number;
  pqcSignatureHex: string;
  pqcPublicKeyHex: string;
}

interface AjnaWasmModule {
  ajna_wasm_create(modelPath: string): number;
  ajna_wasm_process_frame(
    wasmSession: number, rgbaPtr: number,
    width: number, height: number, rustSession: number
  ): void;
  ajna_wasm_destroy(session: number): void;
  ajna_session_create(config: object): number;
  ajna_session_start(session: number, timestampUs: bigint): void;
  ajna_session_get_state(session: number): number;
  ajna_session_get_result_json(session: number, outBuf: number, outBufLen: number): number;
  _malloc(size: number): number;
  _free(ptr: number): void;
  HEAPU8: Uint8Array;
}

export class AjnaScanner {
  private module: AjnaWasmModule | null = null;
  private wasmSession: number = 0;
  private rustSession: number = 0;
  private config: AjnaConfig | null = null;

  async configure(config: AjnaConfig): Promise<void> {
    if (!config.modelPath) throw new Error('AjnaScanner.configure: modelPath is required');
    if (!config.enablePqcSigning) {
      console.warn('[AjnaScanner] enablePqcSigning is false — results will not be PQC-signed');
    }

    // Dynamic import of the Emscripten module (assumed served from same origin as ajna_sdk.js)
    // The module factory is the default export of ajna_sdk.js
    const factory = (await import('./ajna_sdk.js')) as { default: (opts?: object) => Promise<AjnaWasmModule> };
    this.module = await factory.default();

    this.wasmSession = this.module.ajna_wasm_create(config.modelPath);
    if (this.wasmSession === 0) throw new Error('AjnaScanner.configure: failed to create WASM inference session');

    this.rustSession = this.module.ajna_session_create({
      min_quality_frames: 3,
      timeout_ms: 30000,
      adaptive_gate_limit: 60,
      pqc_sign_result: config.enablePqcSigning ?? true,
      include_raw_mrz: false
    });
    this.module.ajna_session_start(this.rustSession, BigInt(Date.now() * 1000));
    this.config = config;
  }

  async processFrame(imageData: ImageData): Promise<{ gateReached: string }> {
    if (!this.module || this.config === null) {
      throw new Error('AjnaScanner.processFrame: call configure() first');
    }
    const { width, height } = imageData;
    const byteCount = width * height * 4;

    // MANDATORY COPY: JS heap to WASM linear memory.
    // This copy is architecturally unavoidable on WASM and is documented as expected cost
    // in the SDK architecture. See platform/wasm/onnx_bridge.cpp module-level note.
    const ptr = this.module._malloc(byteCount);
    try {
      this.module.HEAPU8.set(imageData.data, ptr);
      this.module.ajna_wasm_process_frame(
        this.wasmSession, ptr, width, height, this.rustSession
      );
    } finally {
      this.module._free(ptr);
    }

    const state = this.module.ajna_session_get_state(this.rustSession);
    const stateNames = ['Idle', 'Scanning', 'Inferring', 'Complete', 'Failed'];
    return { gateReached: stateNames[state] ?? 'Unknown' };
  }

  async getScanResult(): Promise<ScanResult | null> {
    if (!this.module || this.config === null) {
      throw new Error('AjnaScanner.getScanResult: call configure() first');
    }
    const state = this.module.ajna_session_get_state(this.rustSession);
    if (state !== 3) return null;  // 3 = Complete

    const BUF_SIZE = 16384;
    const outPtr = this.module._malloc(BUF_SIZE);
    try {
      const written = this.module.ajna_session_get_result_json(
        this.rustSession, outPtr, BUF_SIZE
      );
      if (written <= 0) return null;

      const jsonBytes = this.module.HEAPU8.subarray(outPtr, outPtr + written);
      const jsonStr = new TextDecoder().decode(jsonBytes);
      const parsed = JSON.parse(jsonStr) as {
        fields: Array<{ key: string; value: string; confidence: number }>;
        document_type: string;
        issuing_country: string;
        confidence: number;
        pqc_signature_hex: string;
        pqc_public_key_hex: string;
      };

      return {
        fields: parsed.fields.map(f => ({
          key: f.key, value: f.value, confidence: f.confidence
        })),
        documentType: parsed.document_type,
        issuingCountry: parsed.issuing_country,
        confidence: parsed.confidence,
        pqcSignatureHex: parsed.pqc_signature_hex,
        pqcPublicKeyHex: parsed.pqc_public_key_hex
      };
    } finally {
      this.module._free(outPtr);
    }
  }

  destroy(): void {
    if (!this.module) return;
    if (this.wasmSession !== 0) this.module.ajna_wasm_destroy(this.wasmSession);
    if (this.rustSession !== 0) {
      // ajna_session_destroy is available via the Rust FFI
      // @ts-ignore — function exists in compiled WASM but not typed in AjnaWasmModule interface
      (this.module as Record<string, unknown>)['ajna_session_destroy']?.(this.rustSession);
    }
    this.wasmSession = 0;
    this.rustSession = 0;
    this.module = null;
    this.config = null;
  }
}
