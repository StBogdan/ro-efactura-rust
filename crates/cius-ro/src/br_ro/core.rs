//! Core national CIUS-RO rules (`BR-RO-010`..`BR-RO-040`, `BR-27` fallback) and repeating-group
//! cardinality limits (`BR-RO-A020`, `BR-RO-A050`, `BR-RO-A500`).

use en16931::Invoice;
use roxmltree::Node;

use super::{CII_NS, CII_RAM_NS, UBL_CAC_NS, UBL_CBC_NS, child_elem, children_named};
use crate::stage_acc::StageAcc;
use crate::types::{Locale, ValidateOptions, XmlSyntax};

pub(super) fn check_core_rules(
    inv: &Invoice,
    root: Node<'_, '_>,
    syntax: XmlSyntax,
    opts: &ValidateOptions,
    acc: &mut StageAcc,
) {
    // BR-RO-010: BT-1 must contain at least one ASCII digit
    let num = inv.number.as_deref().unwrap_or("");
    if num.bytes().any(|b| b.is_ascii_digit()) {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => {
                format!(
                    "Invoice number (BT-1) '{num}' must contain at least one numeric digit (0-9)."
                )
            }
            Locale::Ro => {
                format!(
                    "Numărul facturii (BT-1) '{num}' trebuie să conțină cel puțin o cifră (0-9)."
                )
            }
        };
        acc.fatal_sub("BR-RO-010", "BR-RO-010", "BT-1", "BT-1", msg);
    }

    // BR-RO-020: BT-3 allowed type codes (BR-RO-020_1 on UBL Invoice, BR-RO-020_2 on UBL CreditNote)
    let tc = inv.type_code.as_ref().map_or("", |c| c.as_str().trim());
    let (tc_ok, tc_sub) = match syntax {
        XmlSyntax::UblInvoice => (matches!(tc, "380" | "384" | "389" | "751"), "BR-RO-020_1"),
        XmlSyntax::UblCreditNote => (tc == "381", "BR-RO-020_2"),
        XmlSyntax::CiiD16B => (
            matches!(tc, "380" | "381" | "384" | "389" | "751"),
            "BR-RO-020",
        ),
    };
    if tc_ok {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => format!(
                "Invoice type code (BT-3) '{tc}' is not allowed in CIUS-RO (permitted: 380, 384, 389, 751, or 381 for CreditNote)."
            ),
            Locale::Ro => format!(
                "Codul tipului de factură (BT-3) '{tc}' nu este permis în CIUS-RO (valori permise: 380, 384, 389, 751 sau 381 pentru CreditNote)."
            ),
        };
        acc.fatal_sub("BR-RO-020", tc_sub, "BT-3", "BT-3", msg);
    }

    check_foreign_currency_and_tax_point_rules(inv, root, syntax, opts, acc);
}

fn check_foreign_currency_and_tax_point_rules(
    inv: &Invoice,
    root: Node<'_, '_>,
    syntax: XmlSyntax,
    opts: &ValidateOptions,
    acc: &mut StageAcc,
) {
    // BR-RO-030: If DocumentCurrencyCode (BT-5) != RON, TaxCurrencyCode (BT-6) must be RON and BT-111 must be provided
    let doc_ccy = inv.currency.as_ref().map_or("", |c| c.as_str().trim());
    if !doc_ccy.is_empty() && doc_ccy != "RON" {
        let tax_ccy_ron = inv
            .vat_accounting_currency
            .as_ref()
            .is_some_and(|c| c.as_str().trim() == "RON");
        let has_ron_vat =
            inv.totals.vat_total_accounting.is_some() || has_ron_tax_amount_in_xml(root, syntax);
        if tax_ccy_ron && has_ron_vat {
            acc.pass();
        } else {
            let msg = match acc.locale() {
                Locale::En => format!(
                    "When DocumentCurrencyCode (BT-5) is '{doc_ccy}' (other than 'RON'), TaxCurrencyCode (BT-6) must be 'RON' and VAT total in accounting currency (BT-111) must be provided."
                ),
                Locale::Ro => format!(
                    "Dacă moneda facturii (BT-5) este '{doc_ccy}' (alta decât 'RON'), moneda de contabilizare a TVA (BT-6) trebuie să fie 'RON', împreună cu valoarea totală a TVA în RON (BT-111)."
                ),
            };
            acc.fatal_sub(
                "BR-RO-030",
                "BR-RO-030",
                "BT-5 / BT-6 / BT-111",
                "BT-6 / BT-111",
                msg,
            );
        }
    }

    // BR-RO-040: BT-8 VAT point date code in {3, 35, 432} (UBL UNTDID 2005) or {5, 29, 72} (CII DueDateTypeCode UNTDID 2475)
    if syntax == XmlSyntax::CiiD16B {
        if let Some(tx) = child_elem(root, CII_NS, "SupplyChainTradeTransaction")
            && let Some(hs) = child_elem(tx, CII_RAM_NS, "ApplicableHeaderTradeSettlement")
        {
            for tax in children_named(hs, CII_RAM_NS, "ApplicableTradeTax") {
                if let Some(ddtc) = child_elem(tax, CII_RAM_NS, "DueDateTypeCode") {
                    let code = ddtc.text().unwrap_or("").trim();
                    if !code.is_empty() {
                        check_bt8_cii_code(code, "ram:ApplicableTradeTax/ram:DueDateTypeCode", acc);
                    }
                }
            }
        }
    } else if let Some(code) = inv
        .vat_point_date_code
        .as_ref()
        .map(|c| c.as_str().trim())
        .filter(|s| !s.is_empty())
    {
        check_bt8_ubl_code(code, "BT-8", acc);
    }

    // BR-27 fallback when EN 16931 engine is disabled (no duplicate when run_en16931 is true)
    if !opts.run_en16931 {
        for line in &inv.lines {
            if line.price.net_price.into_decimal().is_sign_negative() {
                let msg = acc
                    .tr(
                        "The Item net price (BT-146) shall NOT be negative.",
                        "Prețul net al articolului (BT-146) NU poate fi negativ.",
                    )
                    .to_owned();
                acc.fatal_sub("BR-27", "BR-27", "BT-146", "BG-25/BT-146", msg);
            } else {
                acc.pass();
            }
        }
    }
}

fn check_bt8_ubl_code(code: &str, loc: &str, acc: &mut StageAcc) {
    if matches!(code, "3" | "35" | "432") {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => {
                format!(
                    "Value added tax point date code (BT-8) '{code}' must be one of 3, 35, or 432."
                )
            }
            Locale::Ro => format!(
                "Codul datei exigibilității TVA (BT-8) '{code}' trebuie să fie unul dintre: 3, 35 sau 432."
            ),
        };
        acc.fatal_sub("BR-RO-040", "BR-RO-040", "BT-8", loc, msg);
    }
}

fn check_bt8_cii_code(code: &str, loc: &str, acc: &mut StageAcc) {
    if matches!(code, "5" | "29" | "72") {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => format!(
                "CII DueDateTypeCode (BT-8) '{code}' must be one of 5, 29, or 72 (UNTDID 2475 corresponding to 3, 35, 432)."
            ),
            Locale::Ro => format!(
                "Codul CII DueDateTypeCode (BT-8) '{code}' trebuie să fie unul dintre: 5, 29 sau 72 (UNTDID 2475 aferent codurilor 3, 35, 432)."
            ),
        };
        acc.fatal_sub("BR-RO-040", "BR-RO-040", "BT-8", loc, msg);
    }
}

fn has_ron_tax_amount_in_xml(root: Node<'_, '_>, syntax: XmlSyntax) -> bool {
    match syntax {
        XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote => {
            children_named(root, UBL_CAC_NS, "TaxTotal").any(|tt| {
                child_elem(tt, UBL_CBC_NS, "TaxAmount")
                    .and_then(|ta| ta.attribute("currencyID"))
                    .is_some_and(|c| c.trim() == "RON")
            })
        }
        XmlSyntax::CiiD16B => child_elem(root, CII_NS, "SupplyChainTradeTransaction")
            .and_then(|tx| child_elem(tx, CII_RAM_NS, "ApplicableHeaderTradeSettlement"))
            .and_then(|hs| {
                child_elem(
                    hs,
                    CII_RAM_NS,
                    "SpecifiedTradeSettlementHeaderMonetarySummation",
                )
            })
            .is_some_and(|ms| {
                children_named(ms, CII_RAM_NS, "TaxTotalAmount").any(|ta| {
                    ta.attribute("currencyID")
                        .is_some_and(|c| c.trim() == "RON")
                })
            }),
    }
}

struct CardinalitySpec {
    max: usize,
    code: &'static str,
    sub_id: &'static str,
    bg: &'static str,
    label_en: &'static str,
    label_ro: &'static str,
}

const NOTES_CAP: CardinalitySpec = CardinalitySpec {
    max: 20,
    code: "BR-RO-A020",
    sub_id: "BR-RO-A020",
    bg: "BG-1",
    label_en: "Invoice notes",
    label_ro: "Note în factură",
};

const DOC_REFS_CAP: CardinalitySpec = CardinalitySpec {
    max: 50,
    code: "BR-RO-A050",
    sub_id: "BR-RO-A051",
    bg: "BG-24",
    label_en: "Supporting documents",
    label_ro: "Documente justificative",
};

const ITEM_ATTRS_CAP: CardinalitySpec = CardinalitySpec {
    max: 50,
    code: "BR-RO-A050",
    sub_id: "BR-RO-A052",
    bg: "BG-32",
    label_en: "Item attributes",
    label_ro: "Atribute ale articolului",
};

const PRECEDING_INVOICES_CAP: CardinalitySpec = CardinalitySpec {
    max: 500,
    code: "BR-RO-A500",
    sub_id: "BR-RO-A500",
    bg: "BG-3",
    label_en: "Preceding invoice references",
    label_ro: "Referințe la facturi anterioare",
};

fn check_cap(count: usize, spec: &CardinalitySpec, acc: &mut StageAcc) {
    if count <= spec.max {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => format!(
                "Maximum {} {} ({}) are allowed in CIUS-RO (found {count}).",
                spec.max, spec.label_en, spec.bg
            ),
            Locale::Ro => format!(
                "În CIUS-RO sunt permise maximum {} elemente {} ({}) (găsit: {count}).",
                spec.max, spec.label_ro, spec.bg
            ),
        };
        acc.fatal_sub(spec.code, spec.sub_id, spec.bg, spec.bg, msg);
    }
}

pub(super) fn check_cardinality(inv: &Invoice, syntax: XmlSyntax, acc: &mut StageAcc) {
    check_cap(inv.notes.len(), &NOTES_CAP, acc);
    let doc_refs = inv.attachments.len()
        + usize::from(
            matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote)
                && inv.object_identifier.is_some(),
        );
    check_cap(doc_refs, &DOC_REFS_CAP, acc);
    for line in &inv.lines {
        check_cap(line.item.attributes.len(), &ITEM_ATTRS_CAP, acc);
    }
    check_cap(inv.preceding_invoices.len(), &PRECEDING_INVOICES_CAP, acc);
}
