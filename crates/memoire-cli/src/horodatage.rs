//! Horodatage UTC « AAAAMMJJ-HHMMSSZ » sans dépendance (calendrier grégorien proleptique).

/// Date civile (année, mois, jour) d'un nombre de jours depuis le 1er janvier 1970.
fn date_civile(jours: i64) -> (i64, u32, u32) {
    // Algorithme de H. Hinnant (« days_from_civil » inversé), exact sur tout le calendrier.
    let z = jours + 719_468;
    let ere = z.div_euclid(146_097);
    let jour_ere = z.rem_euclid(146_097);
    let annee_ere = (jour_ere - jour_ere / 1460 + jour_ere / 36_524 - jour_ere / 146_096) / 365;
    let jour_annee = jour_ere - (365 * annee_ere + annee_ere / 4 - annee_ere / 100);
    let mp = (5 * jour_annee + 2) / 153;
    let jour = u32::try_from(jour_annee - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let mois = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let annee = annee_ere + ere * 400 + i64::from(mois <= 2);
    (annee, mois, jour)
}

/// Horodatage UTC d'un instant donné en secondes depuis le 1er janvier 1970.
pub(crate) fn horodatage(secondes: u64) -> String {
    let jours = i64::try_from(secondes / 86_400).unwrap_or(i64::MAX);
    let reste = secondes % 86_400;
    let (annee, mois, jour) = date_civile(jours);
    format!(
        "{annee:04}{mois:02}{jour:02}-{:02}{:02}{:02}Z",
        reste / 3600,
        reste % 3600 / 60,
        reste % 60
    )
}

/// Date d'une fiche de reprise, en UTC à la minute : « AAAA-MM-JJ HH:MM UTC » (FICHE-REPRISE.md,
/// section 2).
pub(crate) fn date_a_la_minute(secondes: u64) -> String {
    let jours = i64::try_from(secondes / 86_400).unwrap_or(i64::MAX);
    let reste = secondes % 86_400;
    let (annee, mois, jour) = date_civile(jours);
    format!(
        "{annee:04}-{mois:02}-{jour:02} {:02}:{:02} UTC",
        reste / 3600,
        reste % 3600 / 60
    )
}

#[cfg(test)]
mod tests {
    use super::{date_a_la_minute, horodatage};

    #[test]
    fn dates_at_the_minute() {
        assert_eq!(date_a_la_minute(0), "1970-01-01 00:00 UTC");
        assert_eq!(date_a_la_minute(951_868_799), "2000-02-29 23:59 UTC");
        assert_eq!(date_a_la_minute(1_792_954_087), "2026-10-25 18:48 UTC");
    }

    // Valeurs de référence calculées indépendamment (module calendar de Python).
    #[test]
    fn reference_values() {
        assert_eq!(horodatage(0), "19700101-000000Z");
        assert_eq!(horodatage(951_868_799), "20000229-235959Z");
        assert_eq!(horodatage(1_792_954_087), "20261025-184807Z");
        assert_eq!(horodatage(4_107_542_400), "21000301-000000Z");
        assert_eq!(horodatage(86_399), "19700101-235959Z");
    }
}
