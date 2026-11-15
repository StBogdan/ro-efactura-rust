//! Data-driven normalized string length checks (`BR-RO-L020`..`BR-RO-L1000`) aligned with
//! official ANAF `RO16931-rules.sch` (`ROeFactura v1.0.9`).

use en16931::invoice::{InvoiceLine, InvoiceNote, ItemAttribute, PaymentMeans, SupportingDocument};
use en16931::{DocumentReference, Identifier, Invoice};

use crate::stage_acc::{StageAcc, tr_fmt};
use crate::types::XmlSyntax;

/// Static descriptor for a single normalized character-length rule (`string-length(normalize-space(.)) <= max_len`).
struct LenRule<T: ?Sized> {
    code: &'static str,
    sub_id: &'static str,
    bt: &'static str,
    max_len: usize,
    extract: fn(&T) -> Option<&str>,
}

const fn rule<T: ?Sized>(
    code: &'static str,
    sub_id: &'static str,
    bt: &'static str,
    max_len: usize,
    extract: fn(&T) -> Option<&str>,
) -> LenRule<T> {
    LenRule {
        code,
        sub_id,
        bt,
        max_len,
        extract,
    }
}

const INVOICE_LEN_RULES: &[LenRule<Invoice>] = &[
    // Document header & references
    rule("BR-RO-L200", "BR-RO-L155", "BT-1", 200, |i| {
        i.number.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1001", "BT-19", 100, |i| {
        i.accounting_reference.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L0302", "BT-12", 200, |i| {
        i.contract_reference.as_ref().map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L0303", "BT-13", 200, |i| {
        i.purchase_order_reference
            .as_ref()
            .map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L0304", "BT-14", 200, |i| {
        i.sales_order_reference
            .as_ref()
            .map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L0305", "BT-15", 200, |i| {
        i.receiving_advice_reference
            .as_ref()
            .map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L0306", "BT-16", 200, |i| {
        i.despatch_advice_reference
            .as_ref()
            .map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L0307", "BT-17", 200, |i| {
        i.tender_reference.as_ref().map(DocumentReference::as_str)
    }),
    rule("BR-RO-L200", "BR-RO-L200", "BT-18", 200, |i| {
        i.object_identifier.as_ref().map(Identifier::content)
    }),
    rule("BR-RO-L300", "BR-RO-L301", "BT-20", 300, |i| {
        i.payment_terms.as_deref()
    }),
    // Seller (BG-4 / BG-5 / BG-6)
    rule("BR-RO-L150", "BR-RO-L151", "BT-35", 150, |i| {
        i.seller.address.line1.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1002", "BT-36", 100, |i| {
        i.seller.address.line2.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1003", "BT-162", 100, |i| {
        i.seller.address.line3.as_deref()
    }),
    rule("BR-RO-L050", "BR-RO-L0501", "BT-37", 50, |i| {
        i.seller.address.city.as_deref()
    }),
    rule("BR-RO-L020", "BR-RO-L0201", "BT-38", 20, |i| {
        i.seller.address.post_code.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1004", "BT-41", 100, |i| {
        i.seller.contact.name.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1005", "BT-42", 100, |i| {
        i.seller.contact.phone.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1006", "BT-43", 100, |i| {
        i.seller.contact.email.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L202", "BT-28", 200, |i| {
        i.seller.trading_name.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L201", "BT-27", 200, |i| {
        i.seller.name.as_deref()
    }),
    rule("BR-RO-L1000", "BR-RO-L1000", "BT-33", 1000, |i| {
        i.seller.additional_legal_information.as_deref()
    }),
    // Buyer (BG-7 / BG-8 / BG-9)
    rule("BR-RO-L150", "BR-RO-L152", "BT-50", 150, |i| {
        i.buyer.address.line1.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1007", "BT-51", 100, |i| {
        i.buyer.address.line2.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1008", "BT-163", 100, |i| {
        i.buyer.address.line3.as_deref()
    }),
    rule("BR-RO-L050", "BR-RO-L0502", "BT-52", 50, |i| {
        i.buyer.address.city.as_deref()
    }),
    rule("BR-RO-L020", "BR-RO-L0202", "BT-53", 20, |i| {
        i.buyer.address.post_code.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1009", "BT-56", 100, |i| {
        i.buyer.contact.name.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1010", "BT-57", 100, |i| {
        i.buyer.contact.phone.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1011", "BT-58", 100, |i| {
        i.buyer.contact.email.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L204", "BT-45", 200, |i| {
        i.buyer.trading_name.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L203", "BT-44", 200, |i| {
        i.buyer.name.as_deref()
    }),
    // Payee (BG-10), TaxRepresentative (BG-11/12), Delivery (BG-13/15)
    rule("BR-RO-L200", "BR-RO-L205", "BT-59", 200, |i| {
        i.payee.as_ref()?.name.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L206", "BT-62", 200, |i| {
        i.tax_representative.as_ref()?.name.as_deref()
    }),
    rule("BR-RO-L150", "BR-RO-L153", "BT-64", 150, |i| {
        i.tax_representative.as_ref()?.address.line1.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1012", "BT-65", 100, |i| {
        i.tax_representative.as_ref()?.address.line2.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1013", "BT-164", 100, |i| {
        i.tax_representative.as_ref()?.address.line3.as_deref()
    }),
    rule("BR-RO-L050", "BR-RO-L0503", "BT-66", 50, |i| {
        i.tax_representative.as_ref()?.address.city.as_deref()
    }),
    rule("BR-RO-L020", "BR-RO-L0203", "BT-67", 20, |i| {
        i.tax_representative.as_ref()?.address.post_code.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L207", "BT-70", 200, |i| {
        i.delivery.as_ref()?.party_name.as_deref()
    }),
    rule("BR-RO-L150", "BR-RO-L154", "BT-75", 150, |i| {
        i.delivery.as_ref()?.address.as_ref()?.line1.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1014", "BT-76", 100, |i| {
        i.delivery.as_ref()?.address.as_ref()?.line2.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1015", "BT-165", 100, |i| {
        i.delivery.as_ref()?.address.as_ref()?.line3.as_deref()
    }),
    rule("BR-RO-L050", "BR-RO-L0504", "BT-77", 50, |i| {
        i.delivery.as_ref()?.address.as_ref()?.city.as_deref()
    }),
    rule("BR-RO-L020", "BR-RO-L0204", "BT-78", 20, |i| {
        i.delivery.as_ref()?.address.as_ref()?.post_code.as_deref()
    }),
    // Payment scalar fields (BG-16 / BG-18)
    rule("BR-RO-L100", "BR-RO-L1016", "BT-82", 100, |i| {
        i.payment.as_ref()?.means_text.as_deref()
    }),
    rule("BR-RO-L140", "BR-RO-L140", "BT-83", 140, |i| {
        i.payment.as_ref()?.remittance_information.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L209", "BT-88", 200, |i| {
        match i.payment.as_ref()?.means.as_ref()? {
            PaymentMeans::Card(card) => card.holder_name.as_deref(),
            _ => None,
        }
    }),
];

const SUPPORTING_DOC_LEN_RULES: &[LenRule<SupportingDocument>] = &[
    rule("BR-RO-L200", "BR-RO-L0308", "BT-122", 200, |d| {
        Some(d.reference.as_str())
    }),
    rule("BR-RO-L100", "BR-RO-L1020", "BT-123", 100, |d| {
        d.description.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L210", "BT-124", 200, |d| {
        d.uri.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L211", "BT-125-2", 200, |d| {
        Some(d.attachment.as_ref()?.filename())
    }),
];

const LINE_LEN_RULES: &[LenRule<InvoiceLine>] = &[
    rule("BR-RO-L100", "BR-RO-L1021", "BT-133", 100, |l| {
        l.accounting_reference.as_deref()
    }),
    rule("BR-RO-L300", "BR-RO-L303", "BT-127", 300, |l| {
        l.note.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1024", "BT-153", 100, |l| {
        l.item.name.as_deref()
    }),
    rule("BR-RO-L200", "BR-RO-L212", "BT-154", 200, |l| {
        l.item.description.as_deref()
    }),
];

const ITEM_ATTR_LEN_RULES: &[LenRule<ItemAttribute>] = &[
    rule("BR-RO-L050", "BR-RO-L0505", "BT-160", 50, |a| {
        a.name.as_deref()
    }),
    rule("BR-RO-L100", "BR-RO-L1025", "BT-161", 100, |a| {
        a.value.as_deref()
    }),
];

fn eval_len_rules<T: ?Sized>(target: &T, rules: &[LenRule<T>], acc: &mut StageAcc) {
    for r in rules {
        check_opt_len((r.extract)(target), r.max_len, r.code, r.sub_id, r.bt, acc);
    }
}

fn check_opt_len(
    val: Option<&str>,
    max: usize,
    code: &str,
    sub: &str,
    bt: &str,
    acc: &mut StageAcc,
) {
    if let Some(raw) = val {
        check_char_len(normalized_char_len(raw), max, code, sub, bt, acc);
    }
}

fn check_char_len(len: usize, max: usize, code: &str, sub: &str, bt: &str, acc: &mut StageAcc) {
    if len <= max {
        acc.pass();
    } else {
        let msg = tr_fmt!(
            acc,
            "Field {bt} exceeds maximum normalized length of {max} characters (found {len}).",
            "Câmpul {bt} depășește lungimea maximă permisă de {max} caractere (găsit: {len})."
        );
        acc.fatal_sub(code, sub, bt, bt, msg);
    }
}

fn check_note_lengths(notes: &[InvoiceNote], syntax: XmlSyntax, acc: &mut StageAcc) {
    for n in notes {
        match (&n.subject_code, &n.note, syntax) {
            (Some(sc), Some(body), XmlSyntax::UblInvoice | XmlSyntax::UblCreditNote) => {
                // UBL encodes `#SubjectCode#NoteBody` in a single cbc:Note element.
                let sc_len = normalized_char_len(sc.as_str());
                let body_len = normalized_char_len(body);
                let leading_ws = usize::from(
                    body.as_bytes()
                        .first()
                        .copied()
                        .is_some_and(is_xpath_whitespace)
                        && body_len > 0,
                );
                check_char_len(
                    2 + sc_len + leading_ws + body_len,
                    300,
                    "BR-RO-L300",
                    "BR-RO-L302",
                    "BT-22",
                    acc,
                );
            }
            (_, Some(body), _) => {
                check_char_len(
                    normalized_char_len(body),
                    300,
                    "BR-RO-L300",
                    "BR-RO-L302",
                    "BT-22",
                    acc,
                );
            }
            (_, None, _) => {}
        }
    }
}

pub(super) fn check_lengths(inv: &Invoice, syntax: XmlSyntax, acc: &mut StageAcc) {
    check_note_lengths(&inv.notes, syntax, acc);
    eval_len_rules(inv, INVOICE_LEN_RULES, acc);

    if let Some(PaymentMeans::CreditTransfer(cts)) =
        inv.payment.as_ref().and_then(|p| p.means.as_ref())
    {
        for ct in cts {
            check_opt_len(
                ct.account_name.as_deref(),
                200,
                "BR-RO-L200",
                "BR-RO-L208",
                "BT-85",
                acc,
            );
        }
    }

    for a in &inv.allowances {
        check_opt_len(
            a.reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1017",
            "BT-97",
            acc,
        );
    }
    for c in &inv.charges {
        check_opt_len(
            c.reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1018",
            "BT-104",
            acc,
        );
    }
    for vb in &inv.vat_breakdown {
        check_opt_len(
            vb.exemption_reason.as_deref(),
            100,
            "BR-RO-L100",
            "BR-RO-L1019",
            "BT-120",
            acc,
        );
    }
    for att in &inv.attachments {
        eval_len_rules(att, SUPPORTING_DOC_LEN_RULES, acc);
    }
    for pi in &inv.preceding_invoices {
        check_opt_len(
            Some(pi.reference.as_str()),
            200,
            "BR-RO-L200",
            "BR-RO-L156",
            "BT-25",
            acc,
        );
    }

    for line in &inv.lines {
        eval_len_rules(line, LINE_LEN_RULES, acc);
        for a in &line.allowances {
            check_opt_len(
                a.reason.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1022",
                "BT-139",
                acc,
            );
        }
        for c in &line.charges {
            check_opt_len(
                c.reason.as_deref(),
                100,
                "BR-RO-L100",
                "BR-RO-L1023",
                "BT-144",
                acc,
            );
        }
        for attr in &line.item.attributes {
            eval_len_rules(attr, ITEM_ATTR_LEN_RULES, acc);
        }
    }
}

const fn is_xpath_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

/// Computes `string-length(normalize-space(s))` per W3C `XPath` 1.0/2.0 (`S ::= #x20 | #x9 | #xD | #xA`)
/// without heap allocation.
fn normalized_char_len(s: &str) -> usize {
    let mut total = 0usize;
    for (idx, word) in s
        .split([' ', '\t', '\r', '\n'])
        .filter(|w| !w.is_empty())
        .enumerate()
    {
        if idx > 0 {
            total += 1;
        }
        total += word.chars().count();
    }
    total
}

#[cfg(test)]
mod tests {
    use super::normalized_char_len;

    #[test]
    fn xpath_normalize_space_only_collapses_xml_whitespace() {
        assert_eq!(normalized_char_len("  abc \t\r\n def  "), 7);
        // Form feed (\x0C), vertical tab (\x0B), and NBSP (\u{00A0}) are NOT XPath S whitespace.
        assert_eq!(normalized_char_len("a\x0Cb"), 3);
        assert_eq!(normalized_char_len("\x0Cabc\x0C"), 5);
        assert_eq!(normalized_char_len("a\x0Bb"), 3);
        assert_eq!(normalized_char_len("a\u{00A0}b"), 3);
    }
}
