//! Identifiants de 128 bits (D3). L'appelant les fournit ; le domaine n'en génère aucun.
//! L'identifiant nul est réservé aux objets initiaux : commit racine, branche main, contexte par défaut.

use std::fmt;

macro_rules! id128 {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; 16]);

        impl $name {
            /// Identifiant construit à partir de ses seize octets.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 16]) -> Self {
                Self(bytes)
            }

            /// Seize octets de l'identifiant.
            #[must_use]
            pub const fn to_bytes(self) -> [u8; 16] {
                self.0
            }

            /// Vrai pour l'identifiant nul, réservé aux objets initiaux.
            #[must_use]
            pub fn is_nil(self) -> bool {
                self.0 == [0; 16]
            }

            /// Analyse la forme canonique `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` en minuscules.
            /// Toute autre écriture est refusée : une identité n'a qu'une forme textuelle.
            #[must_use]
            pub fn parse(text: &str) -> Option<Self> {
                parse_canonical(text).map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write_canonical(&self.0, f)
            }
        }
    };
}

id128!(
    /// Identité d'une note, stable à travers branches et contextes.
    NodeId
);
id128!(
    /// Identité d'une relation.
    EdgeId
);
id128!(
    /// Identité d'un contexte (variante suivie).
    ScopeId
);
id128!(
    /// Identité d'un commit ; c'est celle de l'opération qui l'a publié.
    CommitId
);
id128!(
    /// Identité d'une branche ; c'est celle de l'opération qui l'a créée.
    BranchId
);
id128!(
    /// Identité d'une opération, fournie par l'appelant pour rendre la répétition sûre.
    OpId
);
id128!(
    /// Identité d'un ancrage de note dans le code.
    AnchorId
);

/// Commit racine de tout dépôt.
pub const ROOT: CommitId = CommitId([0; 16]);
/// Branche principale.
pub const MAIN: BranchId = BranchId([0; 16]);
/// Contexte présent dès la création du dépôt.
pub const DEFAULT_SCOPE: ScopeId = ScopeId([0; 16]);

/// Positions des tirets dans la forme canonique.
const HYPHENS: [usize; 4] = [8, 13, 18, 23];

fn write_canonical(bytes: &[u8; 16], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            f.write_str("-")?;
        }
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

fn parse_canonical(text: &str) -> Option<[u8; 16]> {
    let raw = text.as_bytes();
    if raw.len() != 36 || HYPHENS.iter().any(|&i| raw.get(i) != Some(&b'-')) {
        return None;
    }
    let mut digits = raw
        .iter()
        .enumerate()
        .filter(|(i, _)| !HYPHENS.contains(i))
        .map(|(_, c)| lower_hex(*c));
    let mut bytes = [0u8; 16];
    for byte in &mut bytes {
        let high = digits.next()??;
        let low = digits.next()??;
        *byte = high * 16 + low;
    }
    Some(bytes)
}

fn lower_hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    const TEXT: &str = "01234567-89ab-cdef-fedc-ba9876543210";

    #[test]
    fn canonical_form_round_trips() {
        let id = NodeId::from_bytes(SAMPLE);
        assert_eq!(id.to_string(), TEXT);
        assert_eq!(NodeId::parse(TEXT), Some(id));
        assert_eq!(id.to_bytes(), SAMPLE);
    }

    #[test]
    fn only_the_canonical_form_is_accepted() {
        let refused = [
            "01234567-89AB-CDEF-FEDC-BA9876543210",  // majuscules
            "0123456789ab-cdef-fedc-ba98-76543210",  // tirets déplacés
            "01234567-89ab-cdef-fedc-ba98765432100", // trop long
            "01234567-89ab-cdef-fedc-ba987654321",   // trop court
            "01234567-89ab-cdef-fedc-ba98765432g0",  // chiffre non hexadécimal
            "{1234567-89ab-cdef-fedc-ba9876543210",  // caractère hors alphabet
        ];
        for text in refused {
            assert_eq!(NodeId::parse(text), None, "{text}");
        }
    }

    #[test]
    fn nil_is_reserved_for_initial_objects() {
        assert!(ROOT.is_nil() && MAIN.is_nil() && DEFAULT_SCOPE.is_nil());
        assert!(!NodeId::from_bytes(SAMPLE).is_nil());
        assert_eq!(ROOT.to_string(), "00000000-0000-0000-0000-000000000000");
    }
}
