//! Sources immuables et extraits (INV-SOURCE-01). Repris du prototype sans changement de contrat.

use std::sync::Arc;

use crate::error::Error;

/// Source immuable d'un passage ; plages en octets UTF-8, fin exclusive.
/// Le texte rendu par une interface ne doit pas servir de référence à ces plages.
#[derive(Clone, Debug)]
pub struct SourceText(Arc<str>);

impl SourceText {
    /// Conserve le texte tel quel.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(Arc::from(text))
    }

    /// Extrait non vide, borné et aligné sur des caractères UTF-8 complets.
    ///
    /// # Errors
    /// [`Error::InvalidExcerpt`] si la plage est vide, inversée, hors limites ou coupe un caractère.
    pub fn excerpt(&self, start: usize, end: usize) -> Result<Excerpt, Error> {
        if start >= end || self.0.get(start..end).is_none() {
            return Err(Error::InvalidExcerpt);
        }
        Ok(Excerpt {
            source: self.clone(),
            start,
            end,
        })
    }

    /// Texte complet de la source.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Plage validée d'une source ; elle conserve la source en vie.
#[derive(Clone, Debug)]
pub struct Excerpt {
    source: SourceText,
    start: usize,
    end: usize,
}

impl Excerpt {
    /// Texte exact de l'extrait.
    #[must_use]
    pub fn text(&self) -> &str {
        // Plage validée à la construction sur une source immuable : l'indexation ne peut échouer
        // que si cet invariant interne est rompu, ce qui doit alors être visible.
        &self.source.0[self.start..self.end]
    }

    /// Source dont provient l'extrait.
    #[must_use]
    pub fn source(&self) -> &SourceText {
        &self.source
    }
}
