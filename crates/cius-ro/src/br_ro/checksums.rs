//! Romanian fiscal and commercial identifier checksum validation (`BR-RO-CUI`, `BR-RO-CNP`,
//! `BR-RO-ONRC`, `BR-RO-IBAN`) powered by `cius-ro-id`.

use cius_ro_id::{
    Cnp, Onrc, RoIban, is_anaf_b2c_placeholder, is_onrc_candidate, parse_cui_candidate,
};
use en16931::Invoice;
use en16931::invoice::{Party, PaymentMeans};

use super::addr_country;
use crate::stage_acc::{StageAcc, StageFinding, tr_fmt};

/// Stack-allocated bounded set for deduplicating a small slice of candidate identifier strings
/// without heap allocation or index-out-of-bounds panics.
struct SeenSmall<'a, const N: usize> {
    items: [&'a str; N],
    len: usize,
}

impl<'a, const N: usize> SeenSmall<'a, N> {
    const fn new() -> Self {
        Self {
            items: [""; N],
            len: 0,
        }
    }

    fn insert(&mut self, val: &'a str) -> bool {
        if self.items[..self.len].contains(&val) {
            return false;
        }
        if let Some(slot) = self.items.get_mut(self.len) {
            *slot = val;
            self.len += 1;
        }
        true
    }
}

struct ChecksumRoleSpec {
    role_en: &'static str,
    role_ro: &'static str,
    bg: &'static str,
    cui_term: &'static str,
    cnp_term: &'static str,
    onrc_term: &'static str,
}

const SELLER_CHECKSUM_ROLE: ChecksumRoleSpec = ChecksumRoleSpec {
    role_en: "Seller",
    role_ro: "Vânzător",
    bg: "BG-4",
    cui_term: "BG-4 (Romanian CUI)",
    cnp_term: "BG-4 (Romanian CNP)",
    onrc_term: "BG-4 (ONRC)",
};

const BUYER_CHECKSUM_ROLE: ChecksumRoleSpec = ChecksumRoleSpec {
    role_en: "Buyer",
    role_ro: "Cumpărător",
    bg: "BG-7",
    cui_term: "BG-7 (Romanian CUI)",
    cnp_term: "BG-7 (Romanian CNP)",
    onrc_term: "BG-7 (ONRC)",
};

pub(super) fn check_all_checksums(inv: &Invoice, acc: &mut StageAcc) {
    check_party_checksums(&inv.seller, &SELLER_CHECKSUM_ROLE, acc);
    check_party_checksums(&inv.buyer, &BUYER_CHECKSUM_ROLE, acc);

    if let Some(tr_vat) = inv
        .tax_representative
        .as_ref()
        .and_then(|t| t.vat_identifier.as_deref())
        && let Some(res) = parse_cui_candidate(tr_vat)
    {
        match res {
            Ok(_) => acc.pass(),
            Err(e) => {
                let err = e.display(acc.locale());
                let msg = tr_fmt!(
                    acc,
                    "Tax representative fiscal identifier '{tr_vat}' fails CUI validation: {err}",
                    "Identificatorul fiscal al reprezentantului fiscal '{tr_vat}' este invalid: {err}"
                );
                acc.record(
                    StageFinding::warning("BR-RO-CUI", msg)
                        .sub_id("BR-RO-CUI")
                        .term("BT-63")
                        .location("BG-11/BT-63")
                        .expected("Valid modulo-11 Romanian CUI/CIF")
                        .actual(tr_vat.to_owned()),
                );
            }
        }
    }

    if let Some(PaymentMeans::CreditTransfer(cts)) =
        inv.payment.as_ref().and_then(|p| p.means.as_ref())
    {
        for ct in cts {
            if let Some(raw) = ct.account_identifier.as_deref()
                && starts_with_ro_iban_prefix(raw)
            {
                match RoIban::parse(raw) {
                    Ok(_) => acc.pass(),
                    Err(e) => {
                        let err = e.display(acc.locale());
                        let msg = tr_fmt!(
                            acc,
                            "Payee Romanian IBAN '{raw}' (BT-84) is invalid: {err}",
                            "Codul IBAN românesc '{raw}' (BT-84) este invalid: {err}"
                        );
                        acc.record(
                            StageFinding::warning("BR-RO-IBAN", msg)
                                .sub_id("BR-RO-IBAN")
                                .term("BT-84")
                                .location("BG-17/BT-84")
                                .expected("Valid 24-character RO IBAN (ISO 7064 MOD 97-10)")
                                .actual(raw.to_owned()),
                        );
                    }
                }
            }
        }
    }
}

pub(super) fn has_romanian_context(inv: &Invoice) -> bool {
    is_ro_party(&inv.seller)
        || is_ro_party(&inv.buyer)
        || inv.tax_representative.as_ref().is_some_and(|tr| {
            addr_country(&tr.address) == "RO" || has_ro_prefix(tr.vat_identifier.as_deref())
        })
        || inv
            .currency
            .as_ref()
            .is_some_and(|c| c.as_str().trim() == "RON")
        || inv
            .vat_accounting_currency
            .as_ref()
            .is_some_and(|c| c.as_str().trim() == "RON")
}

fn has_ro_prefix(raw: Option<&str>) -> bool {
    raw.is_some_and(|id| {
        id.trim()
            .get(..2)
            .is_some_and(|p| p.eq_ignore_ascii_case("RO"))
    })
}

fn is_ro_party(party: &Party) -> bool {
    addr_country(&party.address) == "RO"
        || has_ro_prefix(party.vat_identifier.as_deref())
        || has_ro_prefix(party.tax_registration.as_deref())
}

fn check_party_checksums(party: &Party, role: &ChecksumRoleSpec, acc: &mut StageAcc) {
    if !is_ro_party(party) {
        return;
    }
    let legal_id = party
        .legal_registration
        .as_ref()
        .map(en16931::Identifier::content);
    let all_candidates = [
        party.vat_identifier.as_deref(),
        party.tax_registration.as_deref(),
        legal_id,
    ];
    check_party_cui_candidates(&all_candidates, role, acc);
    check_party_cnp_candidates(&all_candidates, role, acc);
    check_party_onrc(legal_id, role, acc);
}

fn check_party_cui_candidates(
    all_candidates: &[Option<&str>],
    role: &ChecksumRoleSpec,
    acc: &mut StageAcc,
) {
    let mut seen = SeenSmall::<4>::new();
    for cand in all_candidates.iter().copied().flatten() {
        let trimmed = cand.trim();
        if !seen.insert(trimmed) {
            continue;
        }
        if let Some(res) = parse_cui_candidate(trimmed) {
            match res {
                Ok(_) => acc.pass(),
                Err(e) => {
                    let role_lbl = acc.tr(role.role_en, role.role_ro);
                    let err = e.display(acc.locale());
                    let msg = tr_fmt!(
                        acc,
                        "Romanian {role_lbl} fiscal identifier '{trimmed}' fails modulo-11 verification: {err}",
                        "Identificatorul fiscal al {role_lbl}ului '{trimmed}' a eșuat verificarea cifrei de control modulo-11: {err}"
                    );
                    let hint = acc
                        .tr(
                            "Verify the CUI/CIF against the ANAF / Ministry of Finance register.",
                            "Verificați codul CUI/CIF în registrul contribuabililor ANAF / Ministerul Finanțelor.",
                        )
                        .to_owned();
                    acc.record(
                        StageFinding::warning("BR-RO-CUI", msg)
                            .sub_id("BR-RO-CUI")
                            .term(role.cui_term)
                            .location(role.bg)
                            .expected("Valid modulo-11 Romanian CUI/CIF")
                            .actual(trimmed.to_owned())
                            .hint(hint),
                    );
                }
            }
        }
    }
}

fn check_party_cnp_candidates(
    all_candidates: &[Option<&str>],
    role: &ChecksumRoleSpec,
    acc: &mut StageAcc,
) {
    let mut seen = SeenSmall::<4>::new();
    for cand in all_candidates.iter().copied().flatten() {
        let t = cand.trim();
        if !seen.insert(t) {
            continue;
        }
        if t.len() == 13 && t.bytes().all(|b| b.is_ascii_digit()) {
            if is_anaf_b2c_placeholder(t) {
                acc.pass();
            } else {
                match Cnp::parse(t) {
                    Ok(_) => acc.pass(),
                    Err(e) => {
                        let role_lbl = acc.tr(role.role_en, role.role_ro);
                        let err = e.display(acc.locale());
                        let msg = tr_fmt!(
                            acc,
                            "Romanian {role_lbl} 13-digit identifier '{t}' fails CNP verification: {err}",
                            "Identificatorul de 13 cifre al {role_lbl}ului '{t}' nu este un CNP valid: {err}"
                        );
                        let hint = acc
                            .tr(
                                "Use '0000000000000' for B2C consumers who do not provide a CNP.",
                                "Utilizați '0000000000000' pentru persoane fizice (B2C) care nu comunică CNP-ul.",
                            )
                            .to_owned();
                        acc.record(
                            StageFinding::warning("BR-RO-CNP", msg)
                                .sub_id("BR-RO-CNP")
                                .term(role.cnp_term)
                                .location(role.bg)
                                .expected("Valid 13-digit CNP or 0000000000000 (B2C)")
                                .actual(t.to_owned())
                                .hint(hint),
                        );
                    }
                }
            }
        }
    }
}

fn check_party_onrc(legal_id: Option<&str>, role: &ChecksumRoleSpec, acc: &mut StageAcc) {
    if let Some(legal) = legal_id.filter(|l| is_onrc_candidate(l)) {
        match Onrc::parse(legal) {
            Ok(_) => acc.pass(),
            Err(e) => {
                let role_lbl = acc.tr(role.role_en, role.role_ro);
                let err = e.display(acc.locale());
                let msg = tr_fmt!(
                    acc,
                    "Romanian {role_lbl} Trade Register number '{legal}' is malformed: {err}",
                    "Numărul din Registrul Comerțului (ONRC) '{legal}' al {role_lbl}ului este invalid: {err}"
                );
                acc.record(
                    StageFinding::warning("BR-RO-ONRC", msg)
                        .sub_id("BR-RO-ONRC")
                        .term(role.onrc_term)
                        .location(role.bg)
                        .expected("J40/1234/2022 or J2024001234005")
                        .actual(legal.to_owned()),
                );
            }
        }
    }
}

fn starts_with_ro_iban_prefix(raw: &str) -> bool {
    let mut non_ws = raw.bytes().filter(|b| !b.is_ascii_whitespace());
    matches!(
        (non_ws.next(), non_ws.next()),
        (Some(b'R' | b'r'), Some(b'O' | b'o'))
    )
}
