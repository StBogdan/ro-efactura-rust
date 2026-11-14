export type Locale = "en" | "ro";
export type Severity = "FATAL" | "WARNING" | "INFO";
export type Recommendation = "accept" | "reject";
export type ContainerKind = "XML" | "PDFA3";
export type XmlSyntax = "UblInvoice" | "UblCreditNote" | "CiiD16B";
export type EngineId =
  | "pdfa3_iso19005"
  | "xsd_schema"
  | "en16931_model"
  | "anaf_cius_ro"
  | "gateway";
export type EngineStatus = "PASSED" | "FAILED" | "SKIPPED";
export type RuleCategory =
  | "core"
  | "party_address"
  | "dates"
  | "cardinality"
  | "length"
  | "decimals"
  | "checksum"
  | "container_xsd";
export type IdentifierKind =
  | "cui"
  | "cnp"
  | "anaf_b2c_placeholder"
  | "onrc"
  | "iban"
  | "county"
  | "bucharest_sector"
  | "unknown";

export interface DocumentIdentity {
  invoiceNumber?: string;
  issueDate?: string;
  sellerName?: string;
}

export interface EngineReport {
  id: EngineId;
  name: string;
  version: string;
  status: EngineStatus;
  rulesEvaluated: number;
  passedCount: number;
  failedCount: number;
  warningCount: number;
  skipReason?: string;
}

export interface Violation {
  code: string;
  ruleSubId?: string;
  engine: EngineId;
  severity: Severity;
  businessTerm?: string;
  message: string;
  location?: string;
  expected?: string;
  actual?: string;
  hint?: string;
}

export interface ValidationReport {
  isValid: boolean;
  recommendation: Recommendation;
  locale: Locale;
  container: ContainerKind;
  detectedSyntax?: XmlSyntax;
  customizationId?: string;
  identity?: DocumentIdentity;
  engines: EngineReport[];
  violations: Violation[];
  svrlXml?: string;
}

export interface IdentifierInspection {
  input: string;
  kind: IdentifierKind;
  isValid: boolean;
  normalized?: string;
  details: string;
}

export interface RuleView {
  code: string;
  category: RuleCategory;
  businessTerms: string;
  severity: Severity;
  title: string;
  description: string;
  remediation: string;
}
