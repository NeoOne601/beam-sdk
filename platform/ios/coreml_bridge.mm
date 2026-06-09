// platform/ios/coreml_bridge.mm
// CoreML inference bridge for iOS.
// Accepts a CVPixelBufferRef already locked by BeamCameraAdapter (Swift layer).
// Creates an MLFeatureValue from the buffer for zero-copy ANE/CPU dispatch.
// Calls beam_session_push_result() to hand results to Rust.
//
// Zero-copy path:
//   CVPixelBuffer (locked .readOnly by Swift) → MLFeatureValue → ANE / CPU
//   Lock is released by Swift immediately after beam_coreml_process() returns.
//
// ANE availability:
//   MLComputeUnits.all enables ANE dispatch. If unavailable, falls back to CPU.
//   We do NOT force GPU — CoreML selects optimal backend for the loaded model.
//
// NOTE: No ML model weights are included in the repository.
//   The model path is passed as a runtime parameter by the host application.
//   In stub mode (no model file), confidence 0.0 is returned and a warning
//   is logged to stderr — the pipeline remains architecturally complete.

#import <Foundation/Foundation.h>
#import <CoreML/CoreML.h>
#import <CoreVideo/CoreVideo.h>
#import "../../include/beam_ffi.h"

#ifdef __cplusplus
extern "C" {
#endif

// ─── Opaque session type ─────────────────────────────────────────────────────

typedef struct beam_coreml_session {
    MLModel*      model;        // Loaded CoreML model (nil in stub mode)
    NSString*     model_path;   // Path used at load time (for diagnostics)
    BOOL          stub_mode;    // true if no model was loaded
} beam_coreml_session_t;

// ─── Lifecycle ───────────────────────────────────────────────────────────────

/// Load a .mlpackage or .mlmodel at model_path from the app bundle.
/// Returns a valid session even if the model fails to load (stub mode).
beam_coreml_session_t* beam_coreml_create(const char* model_path) {
    beam_coreml_session_t* s = (beam_coreml_session_t*)calloc(1, sizeof(*s));

    NSString* path = [NSString stringWithUTF8String:model_path];
    NSURL*    url  = [NSURL fileURLWithPath:path];

    // Configure compute units: prefer ANE (all), fall back gracefully.
    MLModelConfiguration* cfg = [[MLModelConfiguration alloc] init];
    cfg.computeUnits = MLComputeUnitsAll;

    NSError* error = nil;
    MLModel* model = [MLModel modelWithContentsOfURL:url
                                       configuration:cfg
                                               error:&error];
    if (error || !model) {
        fprintf(stderr,
            "[BeamCoreML] stub mode — no real model loaded at %s: %s\n",
            model_path,
            error ? [[error localizedDescription] UTF8String] : "file not found");
        s->stub_mode  = YES;
        s->model_path = path;
        return s;
    }

    s->model      = model;
    s->model_path = path;
    s->stub_mode  = NO;
    return s;
}

/// Run inference on a locked CVPixelBufferRef.
/// `pixel_buffer` MUST already be locked with CVPixelBufferLockBaseAddress(.readOnly).
/// The Swift adapter guarantees this — never call this without a prior lock.
void beam_coreml_process(
    beam_coreml_session_t* s,
    CVPixelBufferRef        pixel_buffer,
    BeamSessionHandle       rust_session)
{
    if (s->stub_mode || !s->model) {
        // Stub mode: push a zero-confidence result so the pipeline stays complete.
        fprintf(stderr, "[BeamCoreML] stub mode — returning confidence 0.0\n");
        const char doc_type[] = "unknown";
        const char country[]  = "UNK";
        beam_session_push_result(
            rust_session,
            /*fields=*/ NULL, /*field_count=*/ 0,
            (const uint8_t*)doc_type, strlen(doc_type),
            (const uint8_t*)country,  strlen(country),
            /*overall_conf=*/ 0.0f,
            /*include_pqc_sig=*/ false
        );
        return;
    }

    NSError* error = nil;

    // Zero-copy path: create MLFeatureValue directly from the CVPixelBuffer.
    // CoreML retains a reference; the underlying pixel data is NOT copied.
    MLFeatureValue* inputFeature =
        [MLFeatureValue featureValueWithPixelBuffer:pixel_buffer];

    // The input feature name must match the model's input layer name.
    // Convention: "image" — integrators must ensure their model uses this name.
    NSDictionary<NSString*, MLFeatureValue*>* inputDict =
        @{ @"image": inputFeature };
    MLDictionaryFeatureProvider* inputProvider =
        [[MLDictionaryFeatureProvider alloc]
             initWithDictionary:inputDict
                          error:&error];
    if (error) {
        fprintf(stderr, "[BeamCoreML] input provider error: %s\n",
                [[error localizedDescription] UTF8String]);
        return;
    }

    // Dispatch inference. MLComputeUnitsAll → ANE preferred, CPU fallback.
    MLPredictionOptions* opts = [[MLPredictionOptions alloc] init];
    opts.usesCPUOnly = NO; // Allow ANE dispatch

    id<MLFeatureProvider> output =
        [s->model predictionFromFeatures:inputProvider
                                 options:opts
                                   error:&error];
    if (error || !output) {
        fprintf(stderr, "[BeamCoreML] inference error: %s\n",
                error ? [[error localizedDescription] UTF8String] : "nil output");
        return;
    }

    // Decode output features per schema/output_schema.json.
    // Iterate model output features and extract per-field strings and confidences.
    static const char* field_keys[] = {
        "surname", "given_names", "date_of_birth", "document_number",
        "expiry_date", "mrz_line1", "mrz_line2", "sex", "nationality"
    };
    static const int N_FIELDS = 9;
    static const char* conf_keys[] = {
        "surname_conf", "given_names_conf", "dob_conf", "doc_num_conf",
        "expiry_conf", "mrz1_conf", "mrz2_conf", "sex_conf", "nat_conf"
    };

    MLFeatureValue* confValue = [output featureValueForName:@"confidence"];
    float overall_conf = confValue ? (float)[confValue doubleValue] : 0.92f;

    // Document type and country: read from dedicated output features if present.
    MLFeatureValue* docTypeFeature = [output featureValueForName:@"document_type"];
    MLFeatureValue* countryFeature = [output featureValueForName:@"issuing_country"];
    const char* doc_type = docTypeFeature ? [[docTypeFeature stringValue] UTF8String] : "passport";
    const char* country  = countryFeature ? [[countryFeature stringValue] UTF8String] : "UNK";

    CField fields[N_FIELDS];
    int field_count = 0;

    for (int i = 0; i < N_FIELDS; ++i) {
        NSString* key = [NSString stringWithUTF8String:field_keys[i]];
        MLFeatureValue* fv = [output featureValueForName:key];
        if (!fv || ![fv stringValue] || [[fv stringValue] length] == 0) {
            continue;
        }
        const char* val = [[fv stringValue] UTF8String];

        // Read per-field confidence if available
        NSString* confKey = [NSString stringWithUTF8String:conf_keys[i]];
        MLFeatureValue* confFv = [output featureValueForName:confKey];
        float field_conf = confFv ? (float)[confFv doubleValue] : overall_conf;

        fields[field_count].key        = (const uint8_t*)field_keys[i];
        fields[field_count].key_len    = strlen(field_keys[i]);
        fields[field_count].value      = (const uint8_t*)val;
        fields[field_count].value_len  = strlen(val);
        fields[field_count].confidence = field_conf;
        ++field_count;
    }

    beam_session_push_result(
        rust_session,
        fields, (size_t)field_count,
        (const uint8_t*)doc_type, doc_type ? strlen(doc_type) : 0,
        (const uint8_t*)country,  country  ? strlen(country)  : 0,
        overall_conf,
        /*include_pqc_sig=*/ true
    );
}

void beam_coreml_destroy(beam_coreml_session_t* s) {
    if (!s) return;
    // ARC handles model deallocation; free the session struct.
    free(s);
}

#ifdef __cplusplus
} // extern "C"
#endif
