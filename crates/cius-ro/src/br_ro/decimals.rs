//! Lexical decimal-scale rules (`BR-RO-Z2`: `BR-DEC-RO-*`) across UBL 2.1 and CII D16B XML nodes.

use en16931::Invoice;
use roxmltree::Node;

use super::{
    CII_NS, CII_RAM_NS, CII_UDT_NS, UBL_CAC_NS, UBL_CBC_NS, child_elem, child_text, children_named,
    is_xsd_true,
};
use crate::stage_acc::StageAcc;
use crate::types::{Locale, XmlSyntax};

const UBL_MONETARY_TOTAL_DECIMALS: &[(&str, &str, &str, &str)] = &[
    (
        "LineExtensionAmount",
        "BR-DEC-RO-09",
        "BT-106",
        "cac:LegalMonetaryTotal/cbc:LineExtensionAmount",
    ),
    (
        "AllowanceTotalAmount",
        "BR-DEC-RO-10",
        "BT-107",
        "cac:LegalMonetaryTotal/cbc:AllowanceTotalAmount",
    ),
    (
        "ChargeTotalAmount",
        "BR-DEC-RO-11",
        "BT-108",
        "cac:LegalMonetaryTotal/cbc:ChargeTotalAmount",
    ),
    (
        "TaxExclusiveAmount",
        "BR-DEC-RO-12",
        "BT-109",
        "cac:LegalMonetaryTotal/cbc:TaxExclusiveAmount",
    ),
    (
        "TaxInclusiveAmount",
        "BR-DEC-RO-14",
        "BT-112",
        "cac:LegalMonetaryTotal/cbc:TaxInclusiveAmount",
    ),
    (
        "PrepaidAmount",
        "BR-DEC-RO-16",
        "BT-113",
        "cac:LegalMonetaryTotal/cbc:PrepaidAmount",
    ),
    (
        "PayableRoundingAmount",
        "BR-DEC-RO-17",
        "BT-114",
        "cac:LegalMonetaryTotal/cbc:PayableRoundingAmount",
    ),
    (
        "PayableAmount",
        "BR-DEC-RO-18",
        "BT-115",
        "cac:LegalMonetaryTotal/cbc:PayableAmount",
    ),
];

const CII_MONETARY_SUMMATION_DECIMALS: &[(&str, &str, &str, &str)] = &[
    (
        "LineTotalAmount",
        "BR-DEC-RO-09",
        "BT-106",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:LineTotalAmount",
    ),
    (
        "AllowanceTotalAmount",
        "BR-DEC-RO-10",
        "BT-107",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:AllowanceTotalAmount",
    ),
    (
        "ChargeTotalAmount",
        "BR-DEC-RO-11",
        "BT-108",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:ChargeTotalAmount",
    ),
    (
        "TaxBasisTotalAmount",
        "BR-DEC-RO-12",
        "BT-109",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:TaxBasisTotalAmount",
    ),
    (
        "GrandTotalAmount",
        "BR-DEC-RO-14",
        "BT-112",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:GrandTotalAmount",
    ),
    (
        "TotalPrepaidAmount",
        "BR-DEC-RO-16",
        "BT-113",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:TotalPrepaidAmount",
    ),
    (
        "RoundingAmount",
        "BR-DEC-RO-17",
        "BT-114",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:RoundingAmount",
    ),
    (
        "DuePayableAmount",
        "BR-DEC-RO-18",
        "BT-115",
        "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:DuePayableAmount",
    ),
];

struct AcDecimalRule {
    amt_sub: &'static str,
    amt_bt: &'static str,
    base_sub: &'static str,
    base_bt: &'static str,
}

const DOC_ALLOWANCE_DEC: AcDecimalRule = AcDecimalRule {
    amt_sub: "BR-DEC-RO-01",
    amt_bt: "BT-92",
    base_sub: "BR-DEC-RO-02",
    base_bt: "BT-93",
};
const DOC_CHARGE_DEC: AcDecimalRule = AcDecimalRule {
    amt_sub: "BR-DEC-RO-05",
    amt_bt: "BT-99",
    base_sub: "BR-DEC-RO-06",
    base_bt: "BT-100",
};
const LINE_ALLOWANCE_DEC: AcDecimalRule = AcDecimalRule {
    amt_sub: "BR-DEC-RO-24",
    amt_bt: "BT-136",
    base_sub: "BR-DEC-RO-25",
    base_bt: "BT-137",
};
const LINE_CHARGE_DEC: AcDecimalRule = AcDecimalRule {
    amt_sub: "BR-DEC-RO-27",
    amt_bt: "BT-141",
    base_sub: "BR-DEC-RO-28",
    base_bt: "BT-142",
};

pub(super) fn check_decimals_lexical(
    root: Node<'_, '_>,
    syntax: XmlSyntax,
    inv: &Invoice,
    acc: &mut StageAcc,
) {
    let tax_ccy = inv
        .vat_accounting_currency
        .as_ref()
        .map(|c| c.as_str().trim());
    let doc_ccy = inv.currency.as_ref().map(|c| c.as_str().trim());

    if matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) {
        check_ubl_decimals(
            root,
            syntax == XmlSyntax::UblCreditNote,
            tax_ccy,
            doc_ccy,
            acc,
        );
    } else if let Some(tx) = child_elem(root, CII_NS, "SupplyChainTradeTransaction") {
        check_cii_decimals(tx, tax_ccy, doc_ccy, acc);
    }
}

fn check_decimal_node(
    node: Option<Node<'_, '_>>,
    sub: &str,
    bt: &str,
    loc: &str,
    acc: &mut StageAcc,
) {
    let Some(el) = node else {
        return;
    };
    let raw = el.text().unwrap_or("").trim();
    if raw.is_empty() {
        return;
    }
    if has_valid_max_2_decimals(raw) {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => format!(
                "Monetary amount ({bt}) '{raw}' must be a valid decimal with at most 2 fractional digits under CIUS-RO ({sub})."
            ),
            Locale::Ro => format!(
                "Suma monetară ({bt}) '{raw}' trebuie să fie un număr zecimal valid cu cel mult 2 zecimale conform CIUS-RO ({sub})."
            ),
        };
        acc.fatal_sub("BR-RO-Z2", sub, bt, loc, msg);
    }
}

fn check_ac_pair(
    amt: Option<Node<'_, '_>>,
    base: Option<Node<'_, '_>>,
    rule: &AcDecimalRule,
    amt_loc: &str,
    base_loc: &str,
    acc: &mut StageAcc,
) {
    check_decimal_node(amt, rule.amt_sub, rule.amt_bt, amt_loc, acc);
    check_decimal_node(base, rule.base_sub, rule.base_bt, base_loc, acc);
}

fn check_tax_total_node(
    ta: Node<'_, '_>,
    tax_ccy: Option<&str>,
    doc_ccy: Option<&str>,
    loc: &str,
    acc: &mut StageAcc,
) {
    let ccy = ta.attribute("currencyID").map(str::trim);
    let is_acct = ccy.is_some() && ccy == tax_ccy && tax_ccy != doc_ccy;
    check_decimal_node(
        Some(ta),
        if is_acct {
            "BR-DEC-RO-15"
        } else {
            "BR-DEC-RO-13"
        },
        if is_acct { "BT-111" } else { "BT-110" },
        loc,
        acc,
    );
}

fn check_ubl_decimals(
    root: Node<'_, '_>,
    is_cn: bool,
    tax_ccy: Option<&str>,
    doc_ccy: Option<&str>,
    acc: &mut StageAcc,
) {
    for ac in children_named(root, UBL_CAC_NS, "AllowanceCharge") {
        let is_charge = is_xsd_true(child_text(ac, UBL_CBC_NS, "ChargeIndicator").as_deref());
        check_ac_pair(
            child_elem(ac, UBL_CBC_NS, "Amount"),
            child_elem(ac, UBL_CBC_NS, "BaseAmount"),
            if is_charge {
                &DOC_CHARGE_DEC
            } else {
                &DOC_ALLOWANCE_DEC
            },
            "cac:AllowanceCharge/cbc:Amount",
            "cac:AllowanceCharge/cbc:BaseAmount",
            acc,
        );
    }

    for tt in children_named(root, UBL_CAC_NS, "TaxTotal") {
        if let Some(ta) = child_elem(tt, UBL_CBC_NS, "TaxAmount") {
            check_tax_total_node(ta, tax_ccy, doc_ccy, "cac:TaxTotal/cbc:TaxAmount", acc);
        }
        for sub in children_named(tt, UBL_CAC_NS, "TaxSubtotal") {
            check_decimal_node(
                child_elem(sub, UBL_CBC_NS, "TaxableAmount"),
                "BR-DEC-RO-1009",
                "BT-116",
                "cac:TaxSubtotal/cbc:TaxableAmount",
                acc,
            );
            check_decimal_node(
                child_elem(sub, UBL_CBC_NS, "TaxAmount"),
                "BR-DEC-RO-1010",
                "BT-117",
                "cac:TaxSubtotal/cbc:TaxAmount",
                acc,
            );
        }
    }

    if let Some(lmt) = child_elem(root, UBL_CAC_NS, "LegalMonetaryTotal") {
        for &(tag, sub, bt, loc) in UBL_MONETARY_TOTAL_DECIMALS {
            check_decimal_node(child_elem(lmt, UBL_CBC_NS, tag), sub, bt, loc, acc);
        }
    }

    let (line_tag, line_loc, ac_amt_loc, ac_base_loc) = if is_cn {
        (
            "CreditNoteLine",
            "cac:CreditNoteLine/cbc:LineExtensionAmount",
            "cac:CreditNoteLine/cac:AllowanceCharge/cbc:Amount",
            "cac:CreditNoteLine/cac:AllowanceCharge/cbc:BaseAmount",
        )
    } else {
        (
            "InvoiceLine",
            "cac:InvoiceLine/cbc:LineExtensionAmount",
            "cac:InvoiceLine/cac:AllowanceCharge/cbc:Amount",
            "cac:InvoiceLine/cac:AllowanceCharge/cbc:BaseAmount",
        )
    };
    for line in children_named(root, UBL_CAC_NS, line_tag) {
        check_decimal_node(
            child_elem(line, UBL_CBC_NS, "LineExtensionAmount"),
            "BR-DEC-RO-23",
            "BT-131",
            line_loc,
            acc,
        );
        for ac in children_named(line, UBL_CAC_NS, "AllowanceCharge") {
            let is_charge = is_xsd_true(child_text(ac, UBL_CBC_NS, "ChargeIndicator").as_deref());
            check_ac_pair(
                child_elem(ac, UBL_CBC_NS, "Amount"),
                child_elem(ac, UBL_CBC_NS, "BaseAmount"),
                if is_charge {
                    &LINE_CHARGE_DEC
                } else {
                    &LINE_ALLOWANCE_DEC
                },
                ac_amt_loc,
                ac_base_loc,
                acc,
            );
        }
    }
}

fn check_cii_decimals(
    tx: Node<'_, '_>,
    tax_ccy: Option<&str>,
    doc_ccy: Option<&str>,
    acc: &mut StageAcc,
) {
    if let Some(hs) = child_elem(tx, CII_RAM_NS, "ApplicableHeaderTradeSettlement") {
        for ac in children_named(hs, CII_RAM_NS, "SpecifiedTradeAllowanceCharge") {
            let ind = child_elem(ac, CII_RAM_NS, "ChargeIndicator")
                .and_then(|ci| child_text(ci, CII_UDT_NS, "Indicator"));
            let rule = if is_xsd_true(ind.as_deref()) {
                &DOC_CHARGE_DEC
            } else {
                &DOC_ALLOWANCE_DEC
            };
            check_ac_pair(
                child_elem(ac, CII_RAM_NS, "ActualAmount"),
                child_elem(ac, CII_RAM_NS, "BasisAmount"),
                rule,
                "ram:SpecifiedTradeAllowanceCharge/ram:ActualAmount",
                "ram:SpecifiedTradeAllowanceCharge/ram:BasisAmount",
                acc,
            );
        }
        for tax in children_named(hs, CII_RAM_NS, "ApplicableTradeTax") {
            check_decimal_node(
                child_elem(tax, CII_RAM_NS, "BasisAmount"),
                "BR-DEC-RO-1009",
                "BT-116",
                "ram:ApplicableTradeTax/ram:BasisAmount",
                acc,
            );
            check_decimal_node(
                child_elem(tax, CII_RAM_NS, "CalculatedAmount"),
                "BR-DEC-RO-1010",
                "BT-117",
                "ram:ApplicableTradeTax/ram:CalculatedAmount",
                acc,
            );
        }
        if let Some(ms) = child_elem(
            hs,
            CII_RAM_NS,
            "SpecifiedTradeSettlementHeaderMonetarySummation",
        ) {
            for &(tag, sub, bt, loc) in CII_MONETARY_SUMMATION_DECIMALS {
                check_decimal_node(child_elem(ms, CII_RAM_NS, tag), sub, bt, loc, acc);
            }
            for tta in children_named(ms, CII_RAM_NS, "TaxTotalAmount") {
                check_tax_total_node(
                    tta,
                    tax_ccy,
                    doc_ccy,
                    "ram:SpecifiedTradeSettlementHeaderMonetarySummation/ram:TaxTotalAmount",
                    acc,
                );
            }
        }
    }

    for item in children_named(tx, CII_RAM_NS, "IncludedSupplyChainTradeLineItem") {
        let Some(ls) = child_elem(item, CII_RAM_NS, "SpecifiedLineTradeSettlement") else {
            continue;
        };
        if let Some(lms) = child_elem(
            ls,
            CII_RAM_NS,
            "SpecifiedTradeSettlementLineMonetarySummation",
        ) {
            check_decimal_node(
                child_elem(lms, CII_RAM_NS, "LineTotalAmount"),
                "BR-DEC-RO-23",
                "BT-131",
                "ram:SpecifiedTradeSettlementLineMonetarySummation/ram:LineTotalAmount",
                acc,
            );
        }
        for ac in children_named(ls, CII_RAM_NS, "SpecifiedTradeAllowanceCharge") {
            let ind = child_elem(ac, CII_RAM_NS, "ChargeIndicator")
                .and_then(|ci| child_text(ci, CII_UDT_NS, "Indicator"));
            let rule = if is_xsd_true(ind.as_deref()) {
                &LINE_CHARGE_DEC
            } else {
                &LINE_ALLOWANCE_DEC
            };
            check_ac_pair(
                child_elem(ac, CII_RAM_NS, "ActualAmount"),
                child_elem(ac, CII_RAM_NS, "BasisAmount"),
                rule,
                "ram:SpecifiedLineTradeSettlement/ram:SpecifiedTradeAllowanceCharge/ram:ActualAmount",
                "ram:SpecifiedLineTradeSettlement/ram:SpecifiedTradeAllowanceCharge/ram:BasisAmount",
                acc,
            );
        }
    }
}

fn has_valid_max_2_decimals(raw: &str) -> bool {
    let rest = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    if rest.is_empty() || rest == "." {
        return false;
    }
    match rest.split_once('.') {
        Some((int_part, frac_part)) => {
            int_part.bytes().all(|b| b.is_ascii_digit())
                && frac_part.len() <= 2
                && frac_part.bytes().all(|b| b.is_ascii_digit())
        }
        None => rest.bytes().all(|b| b.is_ascii_digit()),
    }
}

#[cfg(test)]
mod tests {
    use super::has_valid_max_2_decimals;

    #[test]
    fn lexical_max_2_decimals_rejects_malformed_numbers() {
        assert!(has_valid_max_2_decimals("0"));
        assert!(has_valid_max_2_decimals("100"));
        assert!(has_valid_max_2_decimals("100.5"));
        assert!(has_valid_max_2_decimals("100.25"));
        assert!(has_valid_max_2_decimals("-100.25"));
        assert!(has_valid_max_2_decimals("+0.00"));
        assert!(has_valid_max_2_decimals(".50"));
        assert!(has_valid_max_2_decimals("100."));

        assert!(!has_valid_max_2_decimals(""));
        assert!(!has_valid_max_2_decimals("."));
        assert!(!has_valid_max_2_decimals("-."));
        assert!(!has_valid_max_2_decimals("+"));
        assert!(!has_valid_max_2_decimals("100.123"));
        assert!(!has_valid_max_2_decimals("100.12abc"));
        assert!(!has_valid_max_2_decimals("100.1e-5"));
        assert!(!has_valid_max_2_decimals("1e2"));
        assert!(!has_valid_max_2_decimals("100.1.2"));
    }
}
