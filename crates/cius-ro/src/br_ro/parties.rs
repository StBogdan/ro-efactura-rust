//! Party, VAT identifier, postal address, ISO 3166-2:RO county, and Bucharest sector rules
//! (`BR-RO-065`, `BR-RO-080`..`BR-RO-210`).

use cius_ro_id::{BucharestSector, County};
use en16931::Invoice;
use en16931::invoice::PostalAddress;

use super::{addr_country, non_empty};
use crate::stage_acc::{StageAcc, StageFinding};
use crate::types::Locale;

const VAT_ACTIVE_CATEGORIES: &[&str] = &["S", "Z", "E", "AE", "K", "G", "L", "M"];

struct RequiredFieldSpec {
    code: &'static str,
    sub_id: &'static str,
    bt: &'static str,
    loc: &'static str,
    label_en: &'static str,
    label_ro: &'static str,
}

/// Static descriptor for a party/delivery postal address role in CIUS-RO (`Seller`, `Buyer`,
/// `Tax representative`, `Delivery`).
struct PartyRoleSpec {
    role_en: &'static str,
    role_ro_gen: &'static str,
    county_code: &'static str,
    county_sub: &'static str,
    sector_code: &'static str,
    sector_sub: &'static str,
    bt_sub: &'static str,
    bt_city: &'static str,
    loc: &'static str,
    line1_required: RequiredFieldSpec,
    city_required: RequiredFieldSpec,
    subdivision_required: Option<RequiredFieldSpec>,
}

const SELLER_ROLE: PartyRoleSpec = PartyRoleSpec {
    role_en: "Seller",
    role_ro_gen: "Vânzătorului",
    county_code: "BR-RO-110",
    county_sub: "BR-RO-110",
    sector_code: "BR-RO-100",
    sector_sub: "BR-RO-100",
    bt_sub: "BT-39",
    bt_city: "BT-37",
    loc: "BG-4",
    line1_required: RequiredFieldSpec {
        code: "BR-RO-080",
        sub_id: "BR-RO-081",
        bt: "BT-35",
        loc: "BG-5",
        label_en: "Seller address line 1 (BT-35)",
        label_ro: "Linia 1 a adresei Vânzătorului (BT-35)",
    },
    city_required: RequiredFieldSpec {
        code: "BR-RO-090",
        sub_id: "BR-RO-091",
        bt: "BT-37",
        loc: "BG-5",
        label_en: "Seller city (BT-37)",
        label_ro: "Localitatea Vânzătorului (BT-37)",
    },
    subdivision_required: None,
};

const BUYER_ROLE: PartyRoleSpec = PartyRoleSpec {
    role_en: "Buyer",
    role_ro_gen: "Cumpărătorului",
    county_code: "BR-RO-110",
    county_sub: "BR-RO-111",
    sector_code: "BR-RO-100",
    sector_sub: "BR-RO-101",
    bt_sub: "BT-54",
    bt_city: "BT-52",
    loc: "BG-7",
    line1_required: RequiredFieldSpec {
        code: "BR-RO-080",
        sub_id: "BR-RO-082",
        bt: "BT-50",
        loc: "BG-8",
        label_en: "Buyer address line 1 (BT-50)",
        label_ro: "Linia 1 a adresei Cumpărătorului (BT-50)",
    },
    city_required: RequiredFieldSpec {
        code: "BR-RO-090",
        sub_id: "BR-RO-092",
        bt: "BT-52",
        loc: "BG-8",
        label_en: "Buyer city (BT-52)",
        label_ro: "Localitatea Cumpărătorului (BT-52)",
    },
    subdivision_required: None,
};

const TAX_REP_ROLE: PartyRoleSpec = PartyRoleSpec {
    role_en: "Tax representative",
    role_ro_gen: "Reprezentantului fiscal",
    county_code: "BR-RO-170",
    county_sub: "BR-RO-170",
    sector_code: "BR-RO-160",
    sector_sub: "BR-RO-160",
    bt_sub: "BT-68",
    bt_city: "BT-66",
    loc: "BG-12",
    line1_required: RequiredFieldSpec {
        code: "BR-RO-140",
        sub_id: "BR-RO-140",
        bt: "BT-64",
        loc: "BG-12",
        label_en: "Tax representative address line 1 (BT-64)",
        label_ro: "Linia 1 a adresei reprezentantului fiscal (BT-64)",
    },
    city_required: RequiredFieldSpec {
        code: "BR-RO-150",
        sub_id: "BR-RO-150",
        bt: "BT-66",
        loc: "BG-12",
        label_en: "Tax representative city (BT-66)",
        label_ro: "Localitatea reprezentantului fiscal (BT-66)",
    },
    subdivision_required: None,
};

const DELIVERY_ROLE: PartyRoleSpec = PartyRoleSpec {
    role_en: "Delivery",
    role_ro_gen: "de livrare",
    county_code: "BR-RO-210",
    county_sub: "BR-RO-212",
    sector_code: "BR-RO-200",
    sector_sub: "BR-RO-202",
    bt_sub: "BT-79",
    bt_city: "BT-77",
    loc: "BG-15",
    line1_required: RequiredFieldSpec {
        code: "BR-RO-180",
        sub_id: "BR-RO-180",
        bt: "BT-75",
        loc: "BG-15",
        label_en: "Deliver to address line 1 (BT-75)",
        label_ro: "Adresa de livrare — Linia 1 (BT-75)",
    },
    city_required: RequiredFieldSpec {
        code: "BR-RO-200",
        sub_id: "BR-RO-201",
        bt: "BT-77",
        loc: "BG-15",
        label_en: "Deliver to city (BT-77)",
        label_ro: "Localitatea de livrare (BT-77)",
    },
    subdivision_required: Some(RequiredFieldSpec {
        code: "BR-RO-210",
        sub_id: "BR-RO-211",
        bt: "BT-79",
        loc: "BG-15",
        label_en: "Deliver to country subdivision (BT-79)",
        label_ro: "Subdiviziunea țării de livrare (BT-79)",
    }),
};

pub(super) fn check_parties_and_addresses(inv: &Invoice, acc: &mut StageAcc) {
    let uses_active_vat = inv
        .allowances
        .iter()
        .any(|a| VAT_ACTIVE_CATEGORIES.contains(&a.vat.category.as_str()))
        || inv
            .charges
            .iter()
            .any(|c| VAT_ACTIVE_CATEGORIES.contains(&c.vat.category.as_str()))
        || inv
            .vat_breakdown
            .iter()
            .any(|v| VAT_ACTIVE_CATEGORIES.contains(&v.category.as_str()))
        || inv
            .lines
            .iter()
            .any(|l| VAT_ACTIVE_CATEGORIES.contains(&l.vat.category.as_str()));

    if uses_active_vat {
        let has_seller_tax = non_empty(inv.seller.vat_identifier.as_deref())
            || non_empty(inv.seller.tax_registration.as_deref())
            || inv
                .tax_representative
                .as_ref()
                .is_some_and(|tr| non_empty(tr.vat_identifier.as_deref()));
        if has_seller_tax {
            acc.pass();
        } else {
            let msg = acc
                .tr(
                    "Seller tax registration identifier (BT-32), Seller VAT identifier (BT-31), or Seller tax representative VAT identifier (BT-63) shall be present.",
                    "Identificatorul de înregistrare fiscală a Vânzătorului (BT-32) și/sau Identificatorul de TVA al Vânzătorului (BT-31) și/sau al reprezentantului fiscal (BT-63) trebuie să fie înscris.",
                )
                .to_owned();
            acc.fatal_sub(
                "BR-RO-065",
                "BR-RO-065",
                "BT-31 / BT-32 / BT-63",
                "BG-4",
                msg,
            );
        }

        let has_buyer_id = non_empty(inv.buyer.vat_identifier.as_deref())
            || non_empty(inv.buyer.tax_registration.as_deref())
            || inv
                .buyer
                .legal_registration
                .as_ref()
                .is_some_and(|id| !id.is_blank());
        if has_buyer_id {
            acc.pass();
        } else {
            let msg = acc
                .tr(
                    "Buyer legal registration identifier (BT-47) and/or Buyer VAT identifier (BT-48) shall be present.",
                    "Identificatorul de înregistrare legală a Cumpărătorului (BT-47) și/sau Identificatorul de TVA al Cumpărătorului (BT-48) trebuie să fie înscris.",
                )
                .to_owned();
            acc.fatal_sub("BR-RO-120", "BR-RO-120", "BT-47 / BT-48", "BG-7", msg);
        }
    }

    check_party_address(&inv.seller.address, &SELLER_ROLE, acc);
    check_party_address(&inv.buyer.address, &BUYER_ROLE, acc);

    if let Some(tr) = &inv.tax_representative {
        check_party_address(&tr.address, &TAX_REP_ROLE, acc);
    }

    if let Some(a) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) {
        check_party_address(a, &DELIVERY_ROLE, acc);
    }
}

fn check_party_address(addr: &PostalAddress, spec: &PartyRoleSpec, acc: &mut StageAcc) {
    check_required_field(addr.line1.as_deref(), &spec.line1_required, acc);
    check_required_field(addr.city.as_deref(), &spec.city_required, acc);
    if let Some(sub_spec) = &spec.subdivision_required {
        check_required_field(addr.subdivision.as_deref(), sub_spec, acc);
    }
    check_ro_county_and_sector(addr, spec, acc);
}

fn check_required_field(val: Option<&str>, spec: &RequiredFieldSpec, acc: &mut StageAcc) {
    if non_empty(val) {
        acc.pass();
    } else {
        let msg = match acc.locale() {
            Locale::En => format!("{} must be provided.", spec.label_en),
            Locale::Ro => format!("{} trebuie furnizată.", spec.label_ro),
        };
        acc.fatal_sub(spec.code, spec.sub_id, spec.bt, spec.loc, msg);
    }
}

fn check_ro_county_and_sector(addr: &PostalAddress, spec: &PartyRoleSpec, acc: &mut StageAcc) {
    if addr_country(addr) != "RO" {
        return;
    }

    let sub = addr.subdivision.as_deref().map_or("", str::trim);
    if County::from_code(sub).is_some() {
        acc.pass();
    } else {
        let (role_en, role_ro_gen, bt_sub) = (spec.role_en, spec.role_ro_gen, spec.bt_sub);
        let fuzzy = County::lookup_fuzzy(sub);
        let (msg, hint) = match acc.locale() {
            Locale::En => (
                format!(
                    "When {role_en} country code is RO, country subdivision ({bt_sub}) '{sub}' must be coded using ISO 3166-2:RO (e.g. 'RO-B', 'RO-AB'..'RO-VN')."
                ),
                fuzzy.map_or_else(
                    || "Use an ISO 3166-2:RO code such as 'RO-B', 'RO-CJ', 'RO-TM'.".to_owned(),
                    |c| format!("Did you mean '{}' ({})?", c.code(), c.name_ro()),
                ),
            ),
            Locale::Ro => (
                format!(
                    "Dacă țara {role_ro_gen} este RO, subdiviziunea țării ({bt_sub}) '{sub}' trebuie codificată conform listei ISO 3166-2:RO (ex. 'RO-B', 'RO-AB'..'RO-VN')."
                ),
                fuzzy.map_or_else(
                    || {
                        "Utilizați un cod ISO 3166-2:RO valid (ex. 'RO-B', 'RO-CJ', 'RO-TM')."
                            .to_owned()
                    },
                    |c| format!("Înlocuiți valoarea cu '{}' ({}).", c.code(), c.name_ro()),
                ),
            ),
        };
        acc.record(
            StageFinding::fatal(spec.county_code, msg)
                .sub_id(spec.county_sub)
                .term(bt_sub)
                .location(spec.loc)
                .expected("RO-B or RO-AB..RO-VN")
                .actual(sub.to_owned())
                .hint(hint),
        );
    }

    if sub == "RO-B" {
        let city = addr.city.as_deref().map_or("", str::trim);
        if BucharestSector::from_code(city).is_some() {
            acc.pass();
        } else {
            let (role_en, role_ro_gen, bt_city) = (spec.role_en, spec.role_ro_gen, spec.bt_city);
            let fuzzy = BucharestSector::normalize_fuzzy(city);
            let (msg, hint) = match acc.locale() {
                Locale::En => (
                    format!(
                        "When {role_en} country is RO and subdivision is RO-B (Bucharest), city ({bt_city}) must be coded as SECTOR1..SECTOR6 (found '{city}')."
                    ),
                    fuzzy.map_or_else(
                        || {
                            "For Bucharest (RO-B), set CityName to one of SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5, SECTOR6.".to_owned()
                        },
                        |s| format!("Replace '{city}' with exact uppercase '{}'.", s.code()),
                    ),
                ),
                Locale::Ro => (
                    format!(
                        "Dacă țara {role_ro_gen} este RO și subdiviziunea este RO-B (București), localitatea ({bt_city}) trebuie codificată ca SECTOR1..SECTOR6 (găsit: '{city}')."
                    ),
                    fuzzy.map_or_else(
                        || {
                            "Pentru Municipiul București (RO-B), setați CityName la SECTOR1, SECTOR2, SECTOR3, SECTOR4, SECTOR5 sau SECTOR6.".to_owned()
                        },
                        |s| format!("Înlocuiți '{city}' cu valoarea exactă '{}'.", s.code()),
                    ),
                ),
            };
            acc.record(
                StageFinding::fatal(spec.sector_code, msg)
                    .sub_id(spec.sector_sub)
                    .term(bt_city)
                    .location(spec.loc)
                    .expected("SECTOR1..SECTOR6")
                    .actual(city.to_owned())
                    .hint(hint),
            );
        }
    }
}
