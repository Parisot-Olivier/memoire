# Couche service — conception de l’incrément 3.3

Version 0.1 — 24 octobre 2026 — rédigée avant le code (D3, D12, D14).

## 1. Rôle

La crate `memoire-app` est la seule interface de la future CLI (phase 4). Elle fournit ce que le domaine exige de son appelant et que le stockage ne fournit pas :

- **les identités** : UUID de 128 bits pour chaque objet nouveau et chaque opération (D3) ;
- **les enveloppes** : état attendu lu au moment de l’appel, motif fourni par l’utilisateur (D12), identifiant d’opération ;
- **la répétition sûre** : l’appelant peut fournir l’identifiant d’opération d’une tentative interrompue ; la répéter rend le résultat d’origine sans double effet (INV-RETRY-01, déjà durable).

Elle ne contient aucune règle métier et n’écrit jamais en base autrement que par `Store::execute`.

## 2. Source d’identités

```rust
pub trait IdSource {
    fn next(&mut self) -> [u8; 16];
}
```

- `RandomIds` : UUID version 4 (aléatoire, crate `uuid`, D14), employé en usage réel.
- `SequenceIds` : suite déterministe, pour les tests ; jamais l’identifiant nul.
- Une identité tirée n’est jamais réutilisée par le service ; le domaine refuse de toute façon une identité déjà employée (INV-ID-02), ce qui couvre une collision, de probabilité négligeable pour la version 4.

## 3. Interface

```rust
impl<I: IdSource> Service<I> {
    pub fn create(path, ids) -> Result<Self, ServiceError>;
    pub fn open(path, ids) -> Result<Self, ServiceError>;
    pub fn repository(&self) -> &Repository;
    pub fn plan(&mut self, request: Request) -> Planned;          // identités attribuées, aperçu calculé
    pub fn apply(&mut self, planned: Planned, reason: &str) -> Result<Applied, ServiceError>;
    pub fn retry(&mut self, envelope: Envelope) -> Result<Outcome, ServiceError>;
}
```

- `Request` décrit une intention sans identités (créer une note, lier, réfuter…) ; `plan` lui attribue ses identités, construit la commande et calcule l’aperçu (effets, bifurcation), sans rien écrire.
- `apply` exécute l’enveloppe construite au moment du plan. Si le dépôt a changé entre-temps, le domaine la refuse comme périmée : l’interface doit replanifier, jamais appliquer un aperçu obsolète.
- `Applied` rend le résultat, l’enveloppe exacte (pour une répétition éventuelle) et les identités attribuées.

## 4. Tests prévus

- Identités : jamais nulles, jamais deux fois les mêmes ; suite déterministe en test.
- Plan puis application : l’aperçu annoncé égale le résultat ; un plan périmé est refusé sans écriture.
- Répétition : une enveloppe rendue par `apply`, rejouée après réouverture, rend le même résultat.
- Motif obligatoire pour un changement de statut (D12), refusé sans écriture sinon.
- `RandomIds` : identités de version 4, toutes distinctes sur un échantillon.

## 5. Export et import (incrément 3.4)

Rédigé avant le code.

- **Export** : `Service::export(path)` écrit le texte canonique du dépôt complet (FORMAT.md, section 12) dans un fichier destiné à Git. Écriture atomique : fichier temporaire dans le même répertoire, synchronisé sur disque, puis renommé à la place de la cible, puis synchronisation du répertoire. Un export interrompu laisse soit l’ancien fichier, soit le nouveau, jamais un fichier tronqué. Deux exports d’un même dépôt sont identiques à l’octet.
- **Import** : `Service::import(texte, base, ids)` lit et valide d’abord le texte entier (`decode_repository`) ; seulement ensuite, il crée la base, qui ne doit pas exister, et y écrit tous les fragments en une transaction. Un texte invalide est refusé avec sa ligne et son motif, et aucun fichier de base n’est créé.
- Le dépôt importé est égal, champ pour champ, au dépôt exporté : historique, branches, piles de rétablissement, identités employées et journal compris (INV-EXPORT-01, CA-09).

Tests prévus : aller-retour export puis import ; deux exports identiques ; export qui remplace un export précédent ; import refusé dans une base existante ; import d’un texte invalide refusé sans création de fichier ; répétition d’une enveloppe dans le dépôt importé (journal conservé).
