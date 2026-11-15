//! Lexical XML date format checks (`BR-RO-DT001`..`BR-RO-DT006`, `BR-RO-DT003-CN`) across
//! OASIS UBL 2.1 (`YYYY-MM-DD`) and UN/CEFACT CII D16B (`format="102"` `YYYYMMDD`).

use en16931::Date;
use roxmltree::Node;

use super::{CII_NS, CII_RAM_NS, CII_UDT_NS, UBL_CAC_NS, UBL_CBC_NS, child_elem, children_named};
use crate::stage_acc::{StageAcc, tr_fmt};
use crate::types::XmlSyntax;

const UBL_HEADER_DATES: &[(&str, &str, &str, &str)] = &[
    ("IssueDate", "BR-RO-DT001", "BT-2", "cbc:IssueDate"),
    ("TaxPointDate", "BR-RO-DT002", "BT-7", "cbc:TaxPointDate"),
    ("DueDate", "BR-RO-DT003", "BT-9", "cbc:DueDate"),
];

pub(super) fn check_dates_lexical(root: Node<'_, '_>, syntax: XmlSyntax, acc: &mut StageAcc) {
    if matches!(syntax, XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) {
        check_ubl_dates_lexical(root, syntax, acc);
    } else {
        check_cii_dates_lexical(root, acc);
    }
}

fn check_ubl_node_date(
    node: Node<'_, '_>,
    code: &str,
    sub: &str,
    bt: &str,
    loc: &str,
    acc: &mut StageAcc,
) {
    let raw = node.text().unwrap_or("").trim();
    if raw.len() == 10 && Date::parse(raw).is_ok() {
        acc.pass();
    } else {
        let msg = tr_fmt!(
            acc,
            "Date ({bt}) '{raw}' must strictly follow YYYY-MM-DD without timezone offset.",
            "Data ({bt}) '{raw}' trebuie să respecte strict formatul AAAA-LL-ZZ fără fus orar."
        );
        acc.fatal_sub(code, sub, bt, loc, msg);
    }
}

fn check_ubl_dates_lexical(root: Node<'_, '_>, syntax: XmlSyntax, acc: &mut StageAcc) {
    for &(tag, code, bt, loc) in UBL_HEADER_DATES {
        for n in children_named(root, UBL_CBC_NS, tag) {
            check_ubl_node_date(n, code, code, bt, loc, acc);
        }
    }
    if syntax == XmlSyntax::UblCreditNote {
        for pm in children_named(root, UBL_CAC_NS, "PaymentMeans") {
            for n in children_named(pm, UBL_CBC_NS, "PaymentDueDate") {
                check_ubl_node_date(
                    n,
                    "BR-RO-DT003-CN",
                    "BR-RO-DT003_CN",
                    "BT-9",
                    "cac:PaymentMeans/cbc:PaymentDueDate",
                    acc,
                );
            }
        }
    }
    for del in children_named(root, UBL_CAC_NS, "Delivery") {
        for n in children_named(del, UBL_CBC_NS, "ActualDeliveryDate") {
            check_ubl_node_date(
                n,
                "BR-RO-DT004",
                "BR-RO-DT004",
                "BT-72",
                "cac:Delivery/cbc:ActualDeliveryDate",
                acc,
            );
        }
    }
    for ip in children_named(root, UBL_CAC_NS, "InvoicePeriod") {
        check_ubl_period_dates(
            ip,
            "BT-73",
            "BT-74",
            "cac:InvoicePeriod/cbc:StartDate",
            "cac:InvoicePeriod/cbc:EndDate",
            acc,
        );
    }

    let (line_tag, start_loc, end_loc) = if syntax == XmlSyntax::UblCreditNote {
        (
            "CreditNoteLine",
            "cac:CreditNoteLine/cac:InvoicePeriod/cbc:StartDate",
            "cac:CreditNoteLine/cac:InvoicePeriod/cbc:EndDate",
        )
    } else {
        (
            "InvoiceLine",
            "cac:InvoiceLine/cac:InvoicePeriod/cbc:StartDate",
            "cac:InvoiceLine/cac:InvoicePeriod/cbc:EndDate",
        )
    };
    for line in children_named(root, UBL_CAC_NS, line_tag) {
        for ip in children_named(line, UBL_CAC_NS, "InvoicePeriod") {
            check_ubl_period_dates(ip, "BT-134", "BT-135", start_loc, end_loc, acc);
        }
    }
}

fn check_ubl_period_dates(
    ip: Node<'_, '_>,
    start_bt: &str,
    end_bt: &str,
    start_loc: &str,
    end_loc: &str,
    acc: &mut StageAcc,
) {
    for n in children_named(ip, UBL_CBC_NS, "StartDate") {
        check_ubl_node_date(n, "BR-RO-DT005", "BR-RO-DT005", start_bt, start_loc, acc);
    }
    for n in children_named(ip, UBL_CBC_NS, "EndDate") {
        check_ubl_node_date(n, "BR-RO-DT006", "BR-RO-DT006", end_bt, end_loc, acc);
    }
}

fn check_cii_dt(
    parent: Node<'_, '_>,
    expected_leaf: &str,
    code: &str,
    bt: &str,
    loc: &str,
    acc: &mut StageAcc,
) {
    let mut found = false;
    for dt in parent
        .children()
        .filter(|n| n.is_element() && n.tag_name().namespace() == Some(CII_UDT_NS))
    {
        found = true;
        let leaf = dt.tag_name().name();
        let fmt = dt.attribute("format").map(str::trim);
        let raw = dt.text().unwrap_or("").trim();
        if leaf == expected_leaf && fmt == Some("102") && is_valid_cii_date_102(raw) {
            acc.pass();
        } else {
            let fmt_disp = fmt.unwrap_or("");
            let msg = tr_fmt!(
                acc,
                "CII Date ({bt}) '{raw}' (<udt:{leaf}>, format='{fmt_disp}') must use <udt:{expected_leaf} format='102'> and valid YYYYMMDD.",
                "Data CII ({bt}) '{raw}' (<udt:{leaf}>, format='{fmt_disp}') trebuie să aibă <udt:{expected_leaf} format='102'> și o dată validă AAAALLZZ."
            );
            acc.fatal_sub(code, code, bt, loc, msg);
        }
    }
    if !found && parent.children().any(|n| n.is_element()) {
        let msg = tr_fmt!(
            acc,
            "CII Date ({bt}) must contain <udt:{expected_leaf} format='102'> formatted as YYYYMMDD.",
            "Data CII ({bt}) trebuie să conțină <udt:{expected_leaf} format='102'> în format AAAALLZZ."
        );
        acc.fatal_sub(code, code, bt, loc, msg);
    }
}

fn check_cii_dates_lexical(root: Node<'_, '_>, acc: &mut StageAcc) {
    if let Some(ex_doc) = child_elem(root, CII_NS, "ExchangedDocument") {
        for idt in children_named(ex_doc, CII_RAM_NS, "IssueDateTime") {
            check_cii_dt(
                idt,
                "DateTimeString",
                "BR-RO-DT001",
                "BT-2",
                "ram:IssueDateTime/udt:DateTimeString",
                acc,
            );
        }
    }

    let Some(tx) = child_elem(root, CII_NS, "SupplyChainTradeTransaction") else {
        return;
    };

    if let Some(hd) = child_elem(tx, CII_RAM_NS, "ApplicableHeaderTradeDelivery") {
        for ev in children_named(hd, CII_RAM_NS, "ActualDeliverySupplyChainEvent") {
            for odt in children_named(ev, CII_RAM_NS, "OccurrenceDateTime") {
                check_cii_dt(
                    odt,
                    "DateTimeString",
                    "BR-RO-DT004",
                    "BT-72",
                    "ram:OccurrenceDateTime/udt:DateTimeString",
                    acc,
                );
            }
        }
    }

    if let Some(hs) = child_elem(tx, CII_RAM_NS, "ApplicableHeaderTradeSettlement") {
        check_cii_header_settlement_dates(hs, acc);
    }

    for item in children_named(tx, CII_RAM_NS, "IncludedSupplyChainTradeLineItem") {
        if let Some(ls) = child_elem(item, CII_RAM_NS, "SpecifiedLineTradeSettlement") {
            check_cii_billing_period_dates(ls, "BT-134", "BT-135", acc);
        }
    }
}

fn check_cii_header_settlement_dates(hs: Node<'_, '_>, acc: &mut StageAcc) {
    for tax in children_named(hs, CII_RAM_NS, "ApplicableTradeTax") {
        for tpd in children_named(tax, CII_RAM_NS, "TaxPointDate") {
            check_cii_dt(
                tpd,
                "DateString",
                "BR-RO-DT002",
                "BT-7",
                "ram:TaxPointDate/udt:DateString",
                acc,
            );
        }
    }
    for pt in children_named(hs, CII_RAM_NS, "SpecifiedTradePaymentTerms") {
        for dd in children_named(pt, CII_RAM_NS, "DueDateDateTime") {
            check_cii_dt(
                dd,
                "DateTimeString",
                "BR-RO-DT003",
                "BT-9",
                "ram:DueDateDateTime/udt:DateTimeString",
                acc,
            );
        }
    }
    check_cii_billing_period_dates(hs, "BT-73", "BT-74", acc);
}

fn check_cii_billing_period_dates(
    settlement: Node<'_, '_>,
    start_bt: &str,
    end_bt: &str,
    acc: &mut StageAcc,
) {
    for bp in children_named(settlement, CII_RAM_NS, "BillingSpecifiedPeriod") {
        for sdt in children_named(bp, CII_RAM_NS, "StartDateTime") {
            check_cii_dt(
                sdt,
                "DateTimeString",
                "BR-RO-DT005",
                start_bt,
                "ram:StartDateTime/udt:DateTimeString",
                acc,
            );
        }
        for edt in children_named(bp, CII_RAM_NS, "EndDateTime") {
            check_cii_dt(
                edt,
                "DateTimeString",
                "BR-RO-DT006",
                end_bt,
                "ram:EndDateTime/udt:DateTimeString",
                acc,
            );
        }
    }
}

fn is_valid_cii_date_102(raw: &str) -> bool {
    let &[y0, y1, y2, y3, m0, m1, d0, d1] = raw.as_bytes() else {
        return false;
    };
    if ![y0, y1, y2, y3, m0, m1, d0, d1]
        .into_iter()
        .all(|b| b.is_ascii_digit())
    {
        return false;
    }
    let iso_buf = [y0, y1, y2, y3, b'-', m0, m1, b'-', d0, d1];
    core::str::from_utf8(&iso_buf).is_ok_and(|s| Date::parse(s).is_ok())
}
