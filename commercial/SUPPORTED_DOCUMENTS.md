# Beam Verify — Supported Documents

## Document Type Matrix

| Document Type | MRZ Support | Field Extraction | Fraud Detection | Status |
|--------------|-------------|-----------------|-----------------|--------|
| **Passport (TD3)** | ✅ Full (ICAO 9303) | ✅ All fields | ✅ Screen/print detection | GA |
| **Driving Licence** | ⚠️ Varies by country | ✅ Core fields | ✅ Screen/print detection | GA |
| **National ID Card (TD1)** | ✅ Full (ICAO 9303) | ✅ All fields | ✅ Screen/print detection | GA |
| **Residence Permit** | ⚠️ Limited | ✅ Core fields | ✅ Screen/print detection | Beta |

## Extracted Fields

| Field | Key | Format | Available For |
|-------|-----|--------|--------------|
| Surname | `surname` | UTF-8 string | All documents |
| Given names | `given_names` | UTF-8 string | All documents |
| Date of birth | `date_of_birth` | YYYY-MM-DD | All documents |
| Document number | `document_number` | Alphanumeric | All documents |
| Expiry date | `expiry_date` | YYYY-MM-DD | All documents |
| Sex | `sex` | M / F / X | All documents |
| Nationality | `nationality` | ISO 3166-1 alpha-3 | Passports, national IDs |
| MRZ Line 1 | `mrz_line1` | ICAO 9303 | Passports, national IDs |
| MRZ Line 2 | `mrz_line2` | ICAO 9303 | Passports, national IDs |
| Issuing country | `issuing_country` | ISO 3166-1 alpha-3 | All documents |

## MRZ Format Support

| Format | Characters/Line | Lines | Document Types |
|--------|----------------|-------|---------------|
| **TD3** | 44 | 2 | Passports |
| **TD1** | 30 | 3 | National ID cards, residence permits |
| **TD2** | 36 | 2 | Some national ID cards |

## Country Coverage

### Tier 1 — Full validation (tested with real documents)
USA, GBR, DEU, FRA, ESP, ITA, NLD, BEL, AUT, CHE, AUS, CAN, JPN, KOR, SGP, NZL, IRL, PRT, SWE, NOR, DNK, FIN

### Tier 2 — Supported (tested with synthetic documents)
BRA, MEX, ARG, COL, IND, PHL, THA, MYS, IDN, VNM, ZAF, NGA, KEN, EGY, SAU, ARE, QAT, ISR, TUR, POL, CZE, ROU, HUN, GRC, HRV

### Tier 3 — Best-effort (no specific testing)
All other ICAO 9303 compliant documents. MRZ validation works for any compliant document; visual field extraction accuracy may vary.

## Fraud Detection Signals

| Signal | Output Key | Type | Description |
|--------|-----------|------|-------------|
| Screen photo | `is_screen_photo` | float [0, 1] | Probability the document is a photo of a screen |
| Printed fake | `is_printed_fake` | float [0, 1] | Probability the document is a colour photocopy |
| MRZ checksum | `mrz_checksum_valid` | bool | Whether ICAO 9303 check digits validate |

## Limitations

- **NFC chip reading**: Not supported in v1.0. On the Phase 2 roadmap.
- **Barcode/QR code**: PDF417 barcodes on driving licences are not decoded in v1.0.
- **Handwritten text**: Fields requiring handwriting recognition are not supported.
- **Damaged documents**: Heavily worn, folded, or water-damaged documents may fail quality gates.
